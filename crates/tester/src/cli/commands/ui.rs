use crate::cli::args::OutputFormat;
use anyhow::Result;
use std::path::PathBuf;
use tester_core::error::ExitCode;
use tester_core::i18n::{self, Msg};
use tester_lang::adapter::scaffold_playwright;
use tester_lang::discover::discover;

pub(crate) fn cmd_ui_init(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = discover(path)?;
    let root = profile.root.clone();
    scaffold_playwright(&root)?;

    let config_path = root.join("playwright.config.ts");
    let e2e = root.join("e2e");

    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "playwright_config": config_path,
                "e2e": e2e,
                "language": discover(&root)
                    .map(|p| p.language.as_str().to_string())
                    .unwrap_or_else(|_| profile.language.as_str().into()),
            })
        ),
        OutputFormat::Text => {
            println!("{} {}", i18n::t(Msg::PlaywrightInit), root.display());
            println!("{}: {}", i18n::t(Msg::ConfigLabel), config_path.display());
            println!("{}: {}", i18n::t(Msg::TestsLabel), e2e.display());
        }
    }
    Ok(ExitCode::OK)
}

pub(crate) fn cmd_ui_doctor(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let report = tester_pipeline::doctor::run_ui_doctor(path)?;
    match format {
        OutputFormat::Json => println!("{}", report.render_json()),
        OutputFormat::Text => print!("{}", report.render_text()),
    }
    Ok(if report.all_ok() {
        ExitCode::OK
    } else {
        ExitCode::ERROR
    })
}
