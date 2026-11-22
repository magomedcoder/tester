use crate::apply::apply_draft;
use crate::generate::generate_draft;
use crate::plan::{PlanOptions, build_plan_with};
use crate::repair::{LastRepairState, append_history, run_item, save_state};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::path::Path;
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
use tester_core::model::{ItemStatus, ProjectProfile, ReportItem, RunReport, TestKind};
use tester_core::stacktrace::parse_stacktrace;
use tester_llm::Llm;
use tracing::info;

#[derive(Debug, Clone)]
pub struct DebugOptions {
    pub bug_text: String,
    pub reproduce_only: bool,
    pub write: bool,
    pub update: bool,
    pub kind: TestKind,
    pub max_items: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DebugHistoryEntry {
    pub ts_unix: u64,
    pub bug_preview: String,
    pub frames: Vec<String>,
    pub symbols: Vec<String>,
    pub outcome: String,
}

pub fn read_bug_input(bug: Option<String>, bug_file: Option<&Path>, stdin: bool) -> Result<String> {
    if let Some(b) = bug {
        return Ok(b);
    }
    if let Some(path) = bug_file {
        return Ok(std::fs::read_to_string(path)?);
    }
    if stdin || atty_stdin_is_pipe() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        if !buf.trim().is_empty() {
            return Ok(buf);
        }
    }
    bail!("{}", i18n::t(Msg::ErrBugInputRequired));
}

fn atty_stdin_is_pipe() -> bool {
    use std::io::IsTerminal;
    !io::stdin().is_terminal()
}

fn append_debug_history(root: &Path, entry: &DebugHistoryEntry) -> Result<()> {
    let path = root.join(".tester").join("debug-history.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

fn analyze_failure_in(log: &str, root: Option<&Path>) -> String {
    let frames = parse_stacktrace(log);
    let class = tester_core::failure::classify_error(log);
    let mut out = format!("класс_ошибки: {}\n", class.as_str());
    if let Some(root) = root {
        if let Ok(profile) = tester_lang::discover::discover(root) {
            if let Ok(symbols) = tester_lang::index::index_symbols(&profile) {
                let hits = tester_analysis::changed::stacktrace_to_symbols(&symbols, log);
                if !hits.is_empty() {
                    out.push_str("символы_из_стека:\n");
                    for h in hits.iter().take(8) {
                        out.push_str(&format!(
                            "  - {} @ {}:{} ({})\n",
                            h.symbol.as_deref().unwrap_or("?"),
                            h.path.display(),
                            h.start_line,
                            h.why
                        ));
                    }
                }
            }
        }
    }
    if frames.is_empty() {
        out.push_str("фреймы: (не найдены)\n");
        out.push_str("совет: передайте полный traceback в --bug / debug_failure\n");
    } else {
        out.push_str("фреймы:\n");
        for f in frames.iter().take(12) {
            out.push_str(&format!(
                "- {}:{} {}\n",
                f.file,
                f.line.unwrap_or(0),
                f.symbol.as_deref().unwrap_or("?")
            ));
        }
        out.push_str("совет: tester debug . --bug-file <log> --reproduce-only\n");
    }
    out
}

pub fn run_debug(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    config: &Config,
    opts: &DebugOptions,
) -> Result<RunReport> {
    let frames = parse_stacktrace(&opts.bug_text);
    info!(frames = frames.len(), "debug: разобран traceback");

    let plan_opts = PlanOptions {
        diff_base: None,
        bug_text: Some(opts.bug_text.clone()),
        prefer_append: false,
        max_items: Some(if opts.reproduce_only {
            1
        } else {
            opts.max_items.max(1)
        }),
        coverage_path: None,
        stack_frames: frames.clone(),
        reproduce_only: opts.reproduce_only,
    };

    let mut plan = build_plan_with(profile, config, opts.kind, &plan_opts)?;
    if plan.items.is_empty() {
        let mut force = plan_opts.clone();
        force.reproduce_only = true;
        force.max_items = Some(1);
        plan = build_plan_with(profile, config, TestKind::Unit, &force)?;
    }

    if opts.reproduce_only && plan.items.len() > 1 {
        plan.items.truncate(1);
    }

    let mut items = Vec::new();
    for item in &plan.items {
        let mut item = item.clone();
        item.bug_context = Some(opts.bug_text.clone());
        if item.cases.is_empty() || opts.reproduce_only {
            item.cases = vec![tester_core::model::PlannedCase {
                name: format!("{}_repro", item.gap.symbol.name),
                description: "минимальное воспроизведение бага из traceback".into(),
                input: "вход из bug_context".into(),
                expectation: "тест падает на текущем баге или фиксирует исправление".into(),
            }];
        }
        let report = run_item(llm, profile, config, &item, opts.write, opts.update)?;
        items.push(report);
    }

    if items.is_empty() && !plan.items.is_empty() {
        let item = &plan.items[0];
        let draft = generate_draft(llm, profile, config, item)?;
        let _ = apply_draft(profile, &draft, false, true)?;
        items.push(ReportItem {
            symbol: item.gap.symbol.name.clone(),
            kind: item.kind,
            file: draft.path,
            attempts: 0,
            status: ItemStatus::DryRun,
            command: String::new(),
            message: "debug dry-run".into(),
            trace_path: None,
            first_pass: None,
            assert_density: None,
            usefulness: None,
        });
    }

    let symbols: Vec<_> = items.iter().map(|i| i.symbol.clone()).collect();
    let outcome = if items.iter().any(|i| i.status == ItemStatus::Passed) {
        "passed"
    } else if items.iter().any(|i| i.status == ItemStatus::Unfixed) {
        "unfixed"
    } else {
        "planned"
    };
    let _ = append_debug_history(
        &profile.root,
        &DebugHistoryEntry {
            ts_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            bug_preview: opts.bug_text.chars().take(300).collect(),
            frames: frames
                .iter()
                .map(|f| format!("{}:{}:{:?}", f.file, f.line.unwrap_or(0), f.symbol))
                .collect(),
            symbols,
            outcome: outcome.into(),
        },
    );

    if let Some(unfixed) = items.iter().find(|i| i.status == ItemStatus::Unfixed) {
        let _ = save_state(
            &profile.root,
            &LastRepairState {
                root: profile.root.clone(),
                draft: tester_core::model::Draft {
                    path: unfixed.file.clone(),
                    content: std::fs::read_to_string(profile.root.join(&unfixed.file))
                        .unwrap_or_default(),
                    kind: unfixed.kind,
                    symbol_name: unfixed.symbol.clone(),
                    filter_name: String::new(),
                    append: false,
                },
                error: unfixed.message.clone(),
                item_symbol: unfixed.symbol.clone(),
                kind: unfixed.kind,
            },
        );
        let _ = append_history(
            &profile.root,
            &unfixed.symbol,
            ItemStatus::Unfixed,
            &unfixed.message,
        );
    }

    Ok(RunReport {
        items,
        warnings: plan.warnings,
        metrics: Default::default(),
    })
}

pub fn debug_failure_tool_in(root: &Path, log: &str) -> String {
    analyze_failure_in(log, Some(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_panic() {
        let log = r#"
thread 'main' panicked at src/lib.rs:42:5:
called `Result::unwrap()` on an `Err` value
stack backtrace:
   0: demo::multiply
             at ./src/lib.rs:10:5
"#;
        let frames = parse_stacktrace(log);
        assert!(!frames.is_empty(), "{frames:?}");
        assert!(
            frames.iter().any(|f| f.file.contains("lib.rs")),
            "{frames:?}"
        );
    }

    #[test]
    fn analyze_mentions_class() {
        let a = analyze_failure_in("error[E0308]: mismatched types\n", None);
        assert!(a.contains("compile") || a.contains("класс"));
    }
}
