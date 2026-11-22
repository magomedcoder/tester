use crate::cli::args::OutputFormat;
use anyhow::Result;
use std::path::PathBuf;
use tester_core::error::ExitCode;
use tester_core::i18n::{self, Msg};
use tester_lang::discover::discover;
use tester_pipeline::validate::write_bytes;

pub(crate) fn cmd_ui_init(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = discover(path)?;
    let root = profile.root.clone();

    let config_path = root.join("playwright.config.ts");
    if !config_path.exists() {
        let cfg = r#"import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL: 'http://127.0.0.1:3456',
    trace: 'retain-on-failure',
  },
});
"#;
        write_bytes(&config_path, cfg.as_bytes())?;
    }

    let e2e = root.join("e2e");
    std::fs::create_dir_all(&e2e)?;
    let gitkeep = e2e.join(".gitkeep");
    if !gitkeep.exists() {
        write_bytes(&gitkeep, b"")?;
    }

    let pkg_path = root.join("package.json");
    if pkg_path.exists() {
        let text = std::fs::read_to_string(&pkg_path)?;
        let mut pkg: serde_json::Value = serde_json::from_str(&text)?;
        let dev = pkg
            .as_object_mut()
            .unwrap()
            .entry("devDependencies")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(obj) = dev.as_object_mut() {
            obj.entry("@playwright/test".to_string())
                .or_insert_with(|| serde_json::json!("^1.48.0"));
        }
        std::fs::write(&pkg_path, serde_json::to_string_pretty(&pkg)?)?;
    }

    let status = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&root)
        .status();
    if let Ok(st) = status
        && st.success()
    {
        let _ = std::process::Command::new("npx")
            .args(["playwright", "install", "chromium"])
            .current_dir(&root)
            .status();
    }

    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "playwright_config": config_path,
                "e2e": e2e,
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
