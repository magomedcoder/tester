use crate::cli::args::{OutputFormat, SecurityCommands};
use crate::cli::helpers::{load_config, need_llm_with_model, prepare_project};
use anyhow::Result;
use tester_core::error::{AppError, ExitCode};
use tester_core::i18n::{self, Msg};
use tester_security::model::Severity;
use tester_security::{
    draft_patch, explain, render_json, render_markdown, render_text, scan_with_config,
};

pub(crate) fn dispatch_security(cmd: SecurityCommands, model: Option<&str>) -> Result<ExitCode> {
    match cmd {
        SecurityCommands::Scan {
            path,
            deep,
            fetch_advisories,
            fail_on,
            format,
            md,
        } => cmd_scan(&path, deep, fetch_advisories, &fail_on, format, md),
        SecurityCommands::Explain {
            path,
            finding,
            deep,
        } => cmd_explain(&path, &finding, deep),
        SecurityCommands::Patch {
            path,
            finding,
            apply,
            dry_run: _,
            no_dry_run,
            deep,
            llm,
        } => {
            let do_apply = apply || no_dry_run;
            cmd_patch(&path, &finding, do_apply, deep, llm, model)
        }
    }
}

fn cmd_scan(
    path: &std::path::PathBuf,
    deep: bool,
    fetch_advisories: bool,
    fail_on: &str,
    format: OutputFormat,
    md: bool,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let mut config = load_config()?;
    if deep {
        config.security.deep = true;
    }
    if fetch_advisories {
        config.security.fetch_advisories = true;
    }
    let report = scan_with_config(&profile, &config.security, Some(config.security.deep))?;
    if md {
        print!("{}", render_markdown(&report));
    } else {
        match format {
            OutputFormat::Json => println!("{}", render_json(&report)),
            OutputFormat::Text => print!("{}", render_text(&report)),
        }
    }
    let threshold = Severity::parse(fail_on).unwrap_or(Severity::High);
    Ok(ExitCode(tester_security::exit_code(&report, threshold)))
}

fn cmd_explain(path: &std::path::PathBuf, finding: &str, deep: bool) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let mut config = load_config()?;
    if deep {
        config.security.deep = true;
    }
    let report = scan_with_config(&profile, &config.security, Some(config.security.deep))?;
    println!("{}", explain(&report, finding)?);
    Ok(ExitCode::OK)
}

fn cmd_patch(
    path: &std::path::PathBuf,
    finding: &str,
    apply: bool,
    deep: bool,
    use_llm: bool,
    model: Option<&str>,
) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let mut config = load_config()?;
    if deep {
        config.security.deep = true;
    }
    let report = scan_with_config(&profile, &config.security, Some(config.security.deep))?;
    let f = report
        .find(finding)
        .ok_or_else(|| AppError::FindingNotFound {
            id: finding.to_string(),
        })?
        .clone();
    let llm = if use_llm {
        Some(need_llm_with_model(&config, model)?)
    } else {
        None
    };
    let draft = draft_patch(
        &profile,
        &f,
        llm.as_ref().map(|l| l.as_ref() as &dyn tester_providers::Llm),
        apply,
    )?;
    println!(
        "{}",
        i18n::tr(Msg::SecurityPatchNote, &[("note", &draft.note)])
    );
    println!("{}", draft.diff);
    if draft.applied {
        println!(
            "{}",
            i18n::tr(
                Msg::SecurityApplied,
                &[("path", &draft.path.display().to_string())]
            )
        );
    } else if !apply {
        println!("{}", i18n::t(Msg::SecurityDryRunHint));
    }
    Ok(ExitCode::OK)
}
