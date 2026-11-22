#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
)]

mod advisory;
mod patch;
mod redact;
mod report;
mod rules;
mod scanners;
mod walk;

pub mod model;

pub use model::{
    Finding, FixHint, PatchDraft, PatchStrategy, ScanOptions, SecurityReport, Severity,
};
pub use patch::{draft_patch, explain};
pub use report::{exit_code, render_json, render_markdown, render_text};
pub use rules::RULE_PACK_VERSION;

use anyhow::Result;
use tester_core::config::SecurityConfig;
use tester_core::model::ProjectProfile;

pub fn scan(profile: &ProjectProfile, opts: &ScanOptions) -> Result<SecurityReport> {
    let mut notes = Vec::new();
    let mut findings = scanners::run_all(profile, opts, &mut notes)?;
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.start_line.cmp(&b.start_line))
    });
    for (i, f) in findings.iter_mut().enumerate() {
        f.id = format!("F-{:04}", i + 1);
    }
    Ok(SecurityReport {
        root: profile.root.clone(),
        rule_pack_version: RULE_PACK_VERSION.into(),
        deep: opts.deep,
        findings,
        notes,
    })
}

pub fn scan_with_config(
    profile: &ProjectProfile,
    cfg: &SecurityConfig,
    deep_override: Option<bool>,
) -> Result<SecurityReport> {
    if !cfg.enabled {
        return Ok(SecurityReport {
            root: profile.root.clone(),
            rule_pack_version: RULE_PACK_VERSION.into(),
            deep: false,
            findings: Vec::new(),
            notes: vec!["security.enabled=false".into()],
        });
    }
    let opts = ScanOptions::from_config(cfg, deep_override);
    scan(profile, &opts)
}

pub fn rules_loaded_count() -> usize {
    rules::sink_rules().len() + rules::secret_rules().len() + rules::config_rules().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tester_core::model::{Capabilities, Language, TestStyle};

    fn profile(root: &std::path::Path) -> ProjectProfile {
        ProjectProfile {
            root: root.to_path_buf(),
            language: Language::Python,
            package_name: "demo".into(),
            edition: None,
            supported: true,
            test_command: "pytest".into(),
            test_style: TestStyle::Pytest,
            src_dir: Some(root.join("src")),
            tests_dir: None,
            has_existing_tests: false,
            unit_framework: Some("pytest".into()),
            ui_framework: None,
            ui_ready: false,
            capabilities: Capabilities {
                unit: true,
                integration: true,
                contract: false,
                ui: false,
            },
            notes: vec![],
            android_opt_in: false,
            package_filter: None,
        }
    }

    #[test]
    fn finds_os_system_and_patches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut f = std::fs::File::create(dir.path().join("src/app.py")).unwrap();
        writeln!(f, "import os").unwrap();
        writeln!(f, "os.system(user_cmd)").unwrap();
        let p = profile(dir.path());
        let report = scan(&p, &ScanOptions::default()).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|x| x.rule_id.contains("os_system")),
            "{:?}",
            report.findings
        );
        let finding = report
            .findings
            .iter()
            .find(|x| x.rule_id.contains("os_system"))
            .unwrap();
        let draft = draft_patch(&p, finding, None, false).unwrap();
        assert!(draft.diff.contains("subprocess.run") || draft.note.contains("template"));
    }

    #[test]
    fn safe_code_no_critical_rce() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/app.py"),
            "def add(a,b):\n    return a+b\n",
        )
        .unwrap();
        let p = profile(dir.path());
        let report = scan(&p, &ScanOptions::default()).unwrap();
        assert!(
            report
                .findings
                .iter()
                .filter(|f| f.severity == Severity::Critical)
                .count()
                == 0,
            "{:?}",
            report.findings
        );
    }
}
