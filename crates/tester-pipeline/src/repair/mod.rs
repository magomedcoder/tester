use crate::apply::{ApplyOutcome, apply_draft};
use crate::generate::{generate_draft, generate_repair_classified};
use crate::validate::validate_draft;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tester_core::config::Config;
use tester_core::failure::{ErrorClass, classify_error};
use tester_core::gate;
use tester_core::model::{
    Draft, ItemStatus, PlanItem, ProjectProfile, ReportItem, RunReport, TestKind, TestPlan,
};
use tester_lang::green_cache;
use tester_providers::Llm;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct LastRepairState {
    pub root: PathBuf,
    pub draft: Draft,
    pub error: String,
    pub item_symbol: String,
    pub kind: tester_core::model::TestKind,
}

pub fn state_path(root: &std::path::Path) -> PathBuf {
    root.join(".tester").join("last_repair.json")
}

pub fn save_state(root: &std::path::Path, state: &LastRepairState) -> Result<()> {
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct RepairHistoryEntry {
    pub ts_unix: u64,
    pub symbol: String,
    pub status: ItemStatus,
    pub error_preview: String,
}

fn history_path(root: &Path) -> PathBuf {
    root.join(".tester").join("repair-history.jsonl")
}

pub fn append_history(root: &Path, symbol: &str, status: ItemStatus, error: &str) -> Result<()> {
    let path = history_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let entry = RepairHistoryEntry {
        ts_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        symbol: symbol.into(),
        status,
        error_preview: error.chars().take(500).collect(),
    };
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub fn load_state(root: &std::path::Path) -> Result<LastRepairState> {
    let path = state_path(root);
    let text = std::fs::read_to_string(&path).map_err(|_| {
        anyhow!(
            "{}",
            tester_core::i18n::tr(
                tester_core::i18n::Msg::ErrNoRepairState,
                &[("path", &path.display().to_string())],
            )
        )
    })?;
    Ok(serde_json::from_str(&text)?)
}

pub fn run_item(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    config: &Config,
    item: &PlanItem,
    write: bool,
    update: bool,
) -> Result<ReportItem> {
    if !update && green_cache::is_green(&profile.root, &item.gap.symbol.name) {
        return Ok(ReportItem {
            symbol: item.gap.symbol.name.clone(),
            kind: item.kind,
            file: item.target_path.clone(),
            attempts: 0,
            status: ItemStatus::Skipped,
            command: String::new(),
            message: "пропуск: уже зелёный (green-cache); передайте --update".into(),
            trace_path: None,
            first_pass: None,
            assert_density: None,
            usefulness: None,
        });
    }

    let mut draft = generate_draft(llm, profile, config, item)?;
    let density = gate::assert_density(&draft.content);
    let mut attempts = 0u32;

    match apply_draft(profile, &draft, write, update) {
        Err(err) => {
            return Ok(ReportItem {
                symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
                file: draft.path.clone(),
                attempts,
                status: ItemStatus::FailedPath,
                command: String::new(),
                message: err.to_string(),
                trace_path: None,
                first_pass: None,
                assert_density: Some(density),
                usefulness: None,
            });
        }
        Ok(ApplyOutcome::Diff(diff)) => {
            return Ok(ReportItem {
                symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
                file: draft.path.clone(),
                attempts,
                status: ItemStatus::DryRun,
                command: String::new(),
                message: diff,
                trace_path: None,
                first_pass: None,
                assert_density: Some(density),
                usefulness: None,
            });
        }
        Ok(ApplyOutcome::Written(_)) => {}
    }

    loop {
        attempts += 1;
        let mut result = validate_draft(profile, config, &draft)?;
        if !result.ok && attempts == 1 && matches!(item.kind, TestKind::Ui | TestKind::Integration)
        {
            info!("flaky retry validate");
            result = validate_draft(profile, config, &draft)?;
        }
        if result.ok {
            let _ = std::fs::remove_file(state_path(&profile.root));
            let _ = append_history(
                &profile.root,
                &item.gap.symbol.name,
                ItemStatus::Passed,
                "пройден",
            );
            let _ = green_cache::mark_green(&profile.root, &item.gap.symbol.name);
            let first_pass = attempts == 1;
            return Ok(ReportItem {
                symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
                file: draft.path.clone(),
                attempts,
                status: ItemStatus::Passed,
                command: result.command,
                message: "пройден".into(),
                trace_path: result.trace_path,
                first_pass: Some(first_pass),
                assert_density: Some(density),
                usefulness: Some(if first_pass { 1.0 } else { 0.7 }),
            });
        }

        let err_blob = format!("{}\n{}", result.stdout, result.stderr);
        let class = classify_error(&err_blob);
        info!(
            attempt = attempts,
            class = class.as_str(),
            "проверка не прошла"
        );

        if class == ErrorClass::EnvMissing {
            warn!("toolchain/env missing - ранний выход без сжигания repair_attempts");
            return Ok(ReportItem {
                symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
                file: draft.path.clone(),
                attempts,
                status: ItemStatus::Unfixed,
                command: result.command,
                message: format!(
                    "env_missing (ранний выход): {}\nподсказка: tester doctor {}",
                    trim_msg(&err_blob),
                    profile.root.display()
                ),
                trace_path: result.trace_path,
                first_pass: Some(false),
                assert_density: Some(density),
                usefulness: Some(0.0),
            });
        }

        save_state(
            &profile.root,
            &LastRepairState {
                root: profile.root.clone(),
                draft: draft.clone(),
                error: err_blob.clone(),
                item_symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
            },
        )?;
        let _ = append_history(
            &profile.root,
            &item.gap.symbol.name,
            ItemStatus::Unfixed,
            &err_blob,
        );

        if attempts > config.run.repair_attempts {
            warn!("бюджет repair исчерпан");
            return Ok(ReportItem {
                symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
                file: draft.path.clone(),
                attempts,
                status: ItemStatus::Unfixed,
                command: result.command,
                message: trim_msg(&err_blob),
                trace_path: result.trace_path,
                first_pass: Some(false),
                assert_density: Some(density),
                usefulness: Some(0.0),
            });
        }

        draft = generate_repair_classified(llm, profile, item, &draft, &err_blob, class)?;
        apply_draft(profile, &draft, true, true)?;
    }
}

pub fn run_plan(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    config: &Config,
    plan: &TestPlan,
    write: bool,
    update: bool,
) -> Result<Vec<ReportItem>> {
    let concurrency = config.run.concurrency.max(1) as usize;
    if concurrency <= 1 || plan.items.len() <= 1 {
        let mut items = Vec::with_capacity(plan.items.len());
        for item in &plan.items {
            items.push(run_item(llm, profile, config, item, write, update)?);
        }
        return Ok(items);
    }

    let owned: Vec<(usize, PlanItem)> = plan.items.iter().cloned().enumerate().collect();
    let results: Arc<Mutex<Vec<(usize, ReportItem)>>> = Arc::new(Mutex::new(Vec::new()));
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    for batch in owned.chunks(concurrency) {
        std::thread::scope(|scope| {
            for (idx, item) in batch {
                let results = Arc::clone(&results);
                let errors = Arc::clone(&errors);
                let idx = *idx;
                scope.spawn(
                    move || match run_item(llm, profile, config, item, write, update) {
                        Ok(r) => results.lock().unwrap().push((idx, r)),
                        Err(e) => errors.lock().unwrap().push(format!("{e:#}")),
                    },
                );
            }
        });
    }
    if let Some(e) = errors.lock().unwrap().first() {
        return Err(anyhow!("{e}"));
    }
    let mut pairs = results.lock().unwrap().clone();
    pairs.sort_by_key(|(i, _)| *i);
    Ok(pairs.into_iter().map(|(_, r)| r).collect())
}

pub fn enrich_report(report: &mut RunReport) {
    let unit: Vec<_> = report
        .items
        .iter()
        .filter(|i| i.kind == TestKind::Unit)
        .collect();
    if !unit.is_empty() {
        let first = unit.iter().filter(|i| i.first_pass == Some(true)).count() as f64;
        report.metrics.unit_first_pass_rate = Some(first / unit.len() as f64);
    }
    let useful: Vec<f64> = report.items.iter().filter_map(|i| i.usefulness).collect();
    if !useful.is_empty() {
        report.metrics.usefulness_avg = Some(useful.iter().sum::<f64>() / useful.len() as f64);
    }
}

fn trim_msg(s: &str) -> String {
    s.chars().take(2000).collect()
}
