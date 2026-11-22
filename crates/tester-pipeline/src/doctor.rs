use anyhow::Result;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
use tester_core::model::ProjectProfile;
use tester_lang::adapter;
use tester_lang::discover::discover;

#[derive(Debug)]
pub struct DoctorReport {
    pub checks: Vec<(String, bool, String)>,
}

impl DoctorReport {
    pub fn render_text(&self) -> String {
        use tester_core::i18n::{self, Msg};
        let mut out = String::from(i18n::t(Msg::DoctorTitle));
        for (name, ok, detail) in &self.checks {
            out.push_str(&format!(
                "- {} {}: {}\n",
                if *ok {
                    i18n::t(Msg::DoctorOk)
                } else {
                    i18n::t(Msg::DoctorFail)
                },
                name,
                detail
            ));
        }
        out
    }

    pub fn render_json(&self) -> String {
        let checks: Vec<_> = self
            .checks
            .iter()
            .map(|(n, ok, d)| json!({"name": n, "ok": ok, "detail": d}))
            .collect();
        serde_json::to_string_pretty(&json!({ "checks": checks })).unwrap_or_else(|_| "{}".into())
    }

    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(|(_, ok, _)| *ok)
    }
}

pub fn run_doctor(path: &Path) -> Result<DoctorReport> {
    let profile = discover(path)?;
    let config = Config::load(&profile.root)?;
    let mut checks = Vec::new();

    checks.push(check_llm(&config));
    checks.push(check_toolchain(&profile));
    checks.push(check_ui(&profile));
    checks.push((
        "android_opt_in".into(),
        true,
        format!("{}", config.run.android_opt_in),
    ));
    checks.push((
        "allow_docker".into(),
        true,
        format!("{}", config.run.allow_docker),
    ));
    if config.run.allow_docker {
        checks.push(check_docker());
    }
    if config.run.android_opt_in {
        checks.push(check_android(&profile));
    }
    checks.push((
        "security_rules".into(),
        true,
        i18n::tr(
            Msg::DoctorSecurityRules,
            &[
                ("n", &tester_security::rules_loaded_count().to_string()),
                ("enabled", &config.security.enabled.to_string()),
                ("deep", &config.security.deep.to_string()),
            ],
        ),
    ));

    Ok(DoctorReport { checks })
}

pub fn run_ui_doctor(path: &Path) -> Result<DoctorReport> {
    let profile = discover(path)?;
    let config = Config::load(&profile.root)?;
    let mut checks = Vec::new();
    checks.push(check_ui(&profile));
    let base = &config.tests.ui.base_url;
    checks.push(("ui.base_url".into(), !base.is_empty(), base.clone()));
    checks.push((
        "playwright.config".into(),
        profile.root.join("playwright.config.ts").exists()
            || profile.root.join("playwright.config.js").exists(),
        "playwright.config.ts|js".into(),
    ));
    checks.push((
        "e2e_dir".into(),
        profile.root.join("e2e").is_dir() || profile.root.join("tests/e2e").is_dir(),
        "e2e/".into(),
    ));
    let browsers = Command::new("npx")
        .args(["playwright", "--version"])
        .current_dir(&profile.root)
        .output();
    checks.push(match browsers {
        Ok(o) if o.status.success() => (
            "playwright_cli".into(),
            true,
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
        ),
        Ok(o) => (
            "playwright_cli".into(),
            false,
            String::from_utf8_lossy(&o.stderr)
                .chars()
                .take(200)
                .collect(),
        ),
        Err(e) => ("playwright_cli".into(), false, e.to_string()),
    });
    Ok(DoctorReport { checks })
}

fn check_llm(config: &Config) -> (String, bool, String) {
    match tester_llm::list_models(&config.llm) {
        Ok(models) => {
            let selected = config.llm.model.trim();
            let (ok, model) = if !selected.is_empty() {
                (true, selected.to_string())
            } else {
                match models.as_slice() {
                    [] => (false, "(none)".into()),
                    [one] => (true, format!("{one} (auto)")),
                    _ => (false, format!("? ({} available)", models.len())),
                }
            };
            (
                "llm".into(),
                ok,
                format!(
                    "{} model={} ({} listed)",
                    config.llm.base_url,
                    model,
                    models.len()
                ),
            )
        }
        Err(e) => (
            "llm".into(),
            false,
            format!("{e:#} ({})", config.llm.base_url),
        ),
    }
}

fn check_toolchain(profile: &ProjectProfile) -> (String, bool, String) {
    let Ok(adapter) = adapter::adapter_for(profile.language) else {
        return (
            "toolchain".into(),
            false,
            i18n::t(Msg::DoctorNoAdapter).into(),
        );
    };
    let bin = adapter.toolchain_bin(profile);
    let ok = which_ok(bin) || profile.root.join(bin).exists() || bin.starts_with("./");
    (
        "toolchain".into(),
        ok || !bin.is_empty(),
        format!("{} ({})", bin, profile.test_command),
    )
}

fn check_ui(profile: &ProjectProfile) -> (String, bool, String) {
    if !profile.capabilities.ui {
        return ("ui".into(), true, i18n::t(Msg::DoctorNotRequired).into());
    }
    let ready = profile.ui_ready
        || profile.root.join("playwright.config.ts").exists()
        || profile.language == tester_core::model::Language::Flutter;
    (
        "ui".into(),
        ready,
        i18n::tr(
            Msg::DoctorUiState,
            &[
                ("framework", &format!("{:?}", profile.ui_framework)),
                ("ready", &profile.ui_ready.to_string()),
            ],
        ),
    )
}

fn check_docker() -> (String, bool, String) {
    match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(o) if o.status.success() => (
            "docker".into(),
            true,
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
        ),
        Ok(o) => (
            "docker".into(),
            false,
            String::from_utf8_lossy(&o.stderr)
                .chars()
                .take(120)
                .collect(),
        ),
        Err(e) => ("docker".into(), false, e.to_string()),
    }
}

fn check_android(profile: &ProjectProfile) -> (String, bool, String) {
    let gradlew = profile.root.join("gradlew").exists();
    (
        "android".into(),
        gradlew,
        if gradlew {
            i18n::t(Msg::DoctorGradlewFound).into()
        } else {
            i18n::t(Msg::DoctorGradlewMissing).into()
        },
    )
}

fn which_ok(bin: &str) -> bool {
    Command::new("bash")
        .args(["-lc", &format!("command -v {bin}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
