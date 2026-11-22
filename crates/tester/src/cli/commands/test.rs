use crate::cli::args::{OutputFormat, TestCommands};
use crate::cli::helpers::{
    apply_config_flags, emit_report, load_config, need_llm_with_model, parse_kind, plan_opts,
    prepare_project,
};
use anyhow::Result;
use std::path::PathBuf;
use tester_core::config::Config;
use tester_core::error::{AppError, ExitCode};
use tester_core::i18n::{self, Msg};
use tester_core::model::{ItemStatus, RunReport};
use tester_lang::discover::discover;
use tester_pipeline::apply::apply_draft;
use tester_pipeline::eval::{eval_report, eval_suite, render_text as render_eval};
use tester_pipeline::generate::generate_draft;
use tester_pipeline::plan::{PlanOptions, build_plan, build_plan_with};
use tester_pipeline::repair::{LastRepairState, load_state, run_plan, save_state};
use tester_pipeline::report::exit_for;

pub(crate) fn dispatch_test(
    command: TestCommands,
    format: OutputFormat,
    model: Option<&str>,
) -> Result<ExitCode> {
    match command {
        TestCommands::Scan { path } => cmd_scan(&path, format),
        TestCommands::Plan {
            path,
            kind,
            base,
            bug,
            bug_file,
            append,
            package,
            max_items,
            coverage,
            show,
            apply,
        } => {
            if show {
                cmd_plan_show(&path, format)
            } else if apply {
                cmd_plan_apply(&path, false, false, format, model)
            } else {
                cmd_plan(
                    &path,
                    &kind,
                    format,
                    plan_opts(base, bug, bug_file, append, max_items, coverage)?,
                    package,
                )
            }
        }
        TestCommands::Generate {
            path,
            kind,
            write,
            update,
            base,
            bug,
            bug_file,
            append,
            package,
        } => cmd_generate(
            &path,
            &kind,
            write,
            update,
            format,
            plan_opts(base, bug, bug_file, append, None, None)?,
            package,
            model,
        ),
        TestCommands::Run {
            path,
            kind,
            dry_run,
            update,
            base,
            bug,
            bug_file,
            append,
            package,
        } => cmd_run(
            &path,
            &kind,
            !dry_run,
            update,
            format,
            plan_opts(base, bug, bug_file, append, None, None)?,
            package,
            model,
        ),
        TestCommands::Repair { path } => cmd_repair(&path, format, model),
        TestCommands::Eval {
            path,
            max_symbols,
            suite,
            json,
        } => cmd_eval(&path, max_symbols, suite, json, format),
    }
}
pub(crate) fn cmd_scan(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = discover(path)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&profile)?),
        OutputFormat::Text => {
            println!("{}: {}", i18n::t(Msg::Language), profile.language.as_str());
            println!("{}: {}", i18n::t(Msg::Package), profile.package_name);
            println!("{}: {}", i18n::t(Msg::Supported), profile.supported);
            println!("{}: {}", i18n::t(Msg::TestCommand), profile.test_command);
            println!("{}: {:?}", i18n::t(Msg::TestStyle), profile.test_style);
            println!("{}: {}", i18n::t(Msg::HasTests), profile.has_existing_tests);
            if let Some(fw) = &profile.unit_framework {
                println!("{}: {fw}", i18n::t(Msg::UnitFramework));
            }
            if let Some(fw) = &profile.ui_framework {
                println!("{}: {fw}", i18n::t(Msg::UiFramework));
            }
            println!("{}: {}", i18n::t(Msg::UiReady), profile.ui_ready);
            println!(
                "{}: {}",
                i18n::t(Msg::Capabilities),
                profile.capabilities.list().join(", ")
            );
            for n in &profile.notes {
                println!("{}: {n}", i18n::t(Msg::Note));
            }
        }
    }
    Ok(ExitCode::OK)
}
pub(crate) fn cmd_plan(
    path: &PathBuf,
    kind: &str,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>,
) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    let config = Config::load()?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        OutputFormat::Text => {
            if plan.items.is_empty() {
                println!("{}", i18n::t(Msg::NoGoals));
            }
            for item in &plan.items {
                println!(
                    "- [{}] {} {} {} ({})",
                    item.kind.as_str(),
                    item.gap.symbol.name,
                    i18n::t(Msg::PlanArrow),
                    item.target_path.display(),
                    item.gap.reason
                );
            }
            for w in &plan.warnings {
                eprintln!("{}{w}", i18n::t(Msg::WarningPrefix));
            }
        }
    }
    Ok(if plan.items.is_empty() {
        ExitCode::NO_GOALS
    } else {
        ExitCode::OK
    })
}
pub(crate) fn cmd_generate(
    path: &PathBuf,
    kind: &str,
    write: bool,
    update: bool,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>,
    model: Option<&str>,
) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    let config = load_config()?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    if plan.items.is_empty() {
        eprintln!("{}", i18n::t(Msg::NoGoals));
        return Ok(ExitCode::NO_GOALS);
    }
    let llm = need_llm_with_model(&config, model)?;
    let mut report = RunReport {
        warnings: plan.warnings.clone(),
        items: vec![],
        metrics: Default::default(),
    };

    for item in &plan.items {
        match generate_draft(llm.as_ref(), &profile, &config, item) {
            Ok(draft) => match apply_draft(&profile, &draft, write, update) {
                Ok(tester_pipeline::apply::ApplyOutcome::Diff(diff)) => {
                    if matches!(format, OutputFormat::Text) {
                        println!("{diff}");
                    }
                    report.items.push(tester_core::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: draft.path,
                        attempts: 0,
                        status: ItemStatus::DryRun,
                        command: String::new(),
                        message: diff,
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
                Ok(tester_pipeline::apply::ApplyOutcome::Written(p)) => {
                    report.items.push(tester_core::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: p,
                        attempts: 0,
                        status: ItemStatus::Written,
                        command: String::new(),
                        message: i18n::t(Msg::ItemWritten).into(),
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
                Err(err) => {
                    report.items.push(tester_core::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: draft.path,
                        attempts: 0,
                        status: ItemStatus::FailedPath,
                        command: String::new(),
                        message: err.to_string(),
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
            },
            Err(err) => {
                if err.to_string().to_ascii_lowercase().contains("llm")
                    || err.to_string().contains("HTTP")
                    || err.to_string().contains("connect")
                {
                    return Err(err.context(AppError::LlmUnavailable));
                }
                report.warnings.push(err.to_string());
            }
        }
    }

    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}
pub(crate) fn cmd_run(
    path: &PathBuf,
    kind: &str,
    write: bool,
    update: bool,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>,
    model: Option<&str>,
) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    if !profile.supported {
        return Err(AppError::UnsupportedLanguage {
            lang: profile.language.as_str().to_string(),
        }
        .into());
    }
    let config = load_config()?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    if plan.items.is_empty() {
        for w in &plan.warnings {
            eprintln!("{}{w}", i18n::t(Msg::WarningPrefix));
        }
        eprintln!("{}", i18n::t(Msg::NoGoals));
        return Ok(ExitCode::NO_GOALS);
    }
    let llm =
        need_llm_with_model(&config, model).map_err(|e| e.context(AppError::LlmUnavailable))?;
    eprintln!(
        "{}",
        i18n::tr(Msg::PlanProgress, &[("n", &plan.items.len().to_string())])
    );
    let items = run_plan(llm.as_ref(), &profile, &config, &plan, write, update)?;
    let mut report = RunReport {
        items,
        warnings: plan.warnings,
        metrics: Default::default(),
    };
    tester_pipeline::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}
pub(crate) fn cmd_repair(
    path: &PathBuf,
    format: OutputFormat,
    model: Option<&str>,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let config = load_config()?;
    let state = load_state(&profile.root)?;
    let llm = need_llm_with_model(&config, model)?;
    let plan = build_plan(&profile, &config, state.kind)?;
    let item = plan
        .items
        .iter()
        .find(|i| i.gap.symbol.name == state.item_symbol)
        .cloned()
        .ok_or_else(|| AppError::PlanItemNotFound {
            symbol: state.item_symbol.clone(),
        })?;

    let report_item = {
        apply_draft(&profile, &state.draft, true, true)?;

        use tester_pipeline::generate::generate_repair;
        use tester_pipeline::validate::validate_draft;
        let mut draft = state.draft.clone();
        let mut attempts = 0u32;
        let mut last_err = state.error.clone();
        let final_item;
        loop {
            attempts += 1;
            draft = generate_repair(llm.as_ref(), &profile, &item, &draft, &last_err)?;
            apply_draft(&profile, &draft, true, true)?;
            let result = validate_draft(&profile, &config, &draft)?;
            if result.ok {
                let _ = std::fs::remove_file(tester_pipeline::repair::state_path(&profile.root));
                final_item = tester_core::model::ReportItem {
                    symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                    file: draft.path.clone(),
                    attempts,
                    status: ItemStatus::Passed,
                    command: result.command,
                    message: i18n::t(Msg::ItemPassed).into(),
                    trace_path: result.trace_path,
                    first_pass: None,
                    assert_density: None,
                    usefulness: None,
                };
                break;
            }
            last_err = format!("{}\n{}", result.stdout, result.stderr);
            save_state(
                &profile.root,
                &LastRepairState {
                    root: profile.root.clone(),
                    draft: draft.clone(),
                    error: last_err.clone(),
                    item_symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                },
            )?;
            if attempts >= config.run.repair_attempts {
                final_item = tester_core::model::ReportItem {
                    symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                    file: draft.path.clone(),
                    attempts,
                    status: ItemStatus::Unfixed,
                    command: result.command,
                    message: last_err.chars().take(2000).collect(),
                    trace_path: result.trace_path,
                    first_pass: None,
                    assert_density: None,
                    usefulness: None,
                };
                break;
            }
        }
        final_item
    };

    let report = RunReport {
        items: vec![report_item],
        warnings: vec![],
        metrics: Default::default(),
    };
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}
pub(crate) fn cmd_eval(
    path: &PathBuf,
    max_symbols: usize,
    suite: bool,
    as_json: bool,
    format: OutputFormat,
) -> Result<ExitCode> {
    if suite {
        let text = eval_suite(path)?;
        println!("{text}");
        return Ok(ExitCode::OK);
    }
    let report = eval_report(path, max_symbols, true)?;
    if as_json || matches!(format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render_eval(&report));
    }
    Ok(ExitCode::OK)
}
pub(crate) fn cmd_plan_show(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let plan = tester_pipeline::plan_store::load_plan(&profile.root)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        OutputFormat::Text => {
            for item in &plan.items {
                println!(
                    "- [{}] {} -> {}",
                    item.kind.as_str(),
                    item.gap.symbol.name,
                    item.target_path.display()
                );
            }
            if plan.items.is_empty() {
                println!("{}", i18n::t(Msg::PlanShowEmpty));
            }
        }
    }
    Ok(ExitCode(if plan.items.is_empty() { 2 } else { 0 }))
}
pub(crate) fn cmd_plan_apply(
    path: &PathBuf,
    write: bool,
    update: bool,
    format: OutputFormat,
    model: Option<&str>,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let config = load_config()?;
    let plan = tester_pipeline::plan_store::load_plan(&profile.root)?;
    if plan.items.is_empty() {
        return Ok(ExitCode::NO_GOALS);
    }
    let llm = need_llm_with_model(&config, model)?;
    let items = run_plan(llm.as_ref(), &profile, &config, &plan, write, update)?;
    let mut report = RunReport {
        items,
        warnings: plan.warnings,
        metrics: Default::default(),
    };
    tester_pipeline::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}
