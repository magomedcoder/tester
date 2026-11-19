use super::{LanguageAdapter, base_profile};
use crate::index;
use crate::model::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("Cargo.toml").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let cargo = root.join("Cargo.toml");
        let text = std::fs::read_to_string(&cargo)
            .with_context(|| format!("чтение {}", cargo.display()))?;
        let value: toml::Value = toml::from_str(&text)?;

        let package_name = value
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("crate")
            .to_string();
        let edition = value
            .get("package")
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .map(|s| s.to_string());

        let src_dir = root.join("src");
        let tests_dir = root.join("tests");
        let has_inline = src_has_cfg_test(&src_dir);
        let has_tests_dir = tests_dir.is_dir()
            && std::fs::read_dir(&tests_dir)
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .any(|e| e.path().extension().is_some_and(|x| x == "rs"))
                })
                .unwrap_or(false);

        let test_style = if has_inline {
            TestStyle::InlineCfgTest
        } else if has_tests_dir {
            TestStyle::SeparateTestsDir
        } else {
            TestStyle::InlineCfgTest
        };

        Ok(ProjectProfile {
            root: root.to_path_buf(),
            language: Language::Rust,
            package_name,
            edition,
            supported: true,
            test_command: "cargo test".into(),
            test_style,
            src_dir: src_dir.is_dir().then_some(src_dir),
            tests_dir: tests_dir.is_dir().then_some(tests_dir),
            has_existing_tests: has_inline || has_tests_dir,
            unit_framework: Some("cargo".into()),
            ui_framework: None,
            ui_ready: false,
            capabilities: Capabilities {
                unit: true,
                integration: true,
                contract: true,
                ui: false,
            },
            notes: vec![],
            android_opt_in: false,
            package_filter: None,
        })
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        index::rust::index_rust(&profile.root)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn is_test_file(&self, path: &Path, text: &str) -> bool {
        path.components().any(|c| c.as_os_str() == "tests")
            || text.contains("#[test]")
            || text.contains("#[tokio::test]")
            || text.contains("#[rstest]")
    }

    fn suggest_path(&self, _profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf {
        match kind {
            TestKind::Unit => PathBuf::from("tests").join(format!("{}_unit.rs", symbol.name)),
            TestKind::Integration | TestKind::Contract => {
                PathBuf::from("tests").join(format!("{}_test.rs", symbol.name))
            }
            TestKind::Ui => PathBuf::from("tests").join(format!("{}_ui.rs", symbol.name)),
            TestKind::All => PathBuf::from("tests").join(format!("{}_test.rs", symbol.name)),
        }
    }

    fn path_allowed(&self, _profile: &ProjectProfile, kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        if s == "src/lib.rs" || s == "src/main.rs" || s.contains("Cargo.toml") {
            return false;
        }
        match kind {
            TestKind::Ui => false,
            _ => {
                s.starts_with("tests/")
                    || (s.starts_with("src/") && (s.contains("test") || s.ends_with("_test.rs")))
            }
        }
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let mut args = vec!["cargo".into(), "test".into()];
        if let Some(pkg) = &profile.package_filter {
            args.push("-p".into());
            args.push(pkg.clone());
        }
        args.push(draft.filter_name.clone());
        args.push("--".into());
        args.push("--nocapture".into());
        args
    }

    fn capabilities(&self, _profile: &ProjectProfile) -> Capabilities {
        Capabilities {
            unit: true,
            integration: true,
            contract: true,
            ui: false,
        }
    }

    fn toolchain_bin(&self, _profile: &ProjectProfile) -> &'static str {
        "cargo"
    }
}

fn src_has_cfg_test(src: &Path) -> bool {
    if !src.is_dir() {
        return false;
    }
    walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .any(|e| {
            std::fs::read_to_string(e.path())
                .map(|t| t.contains("#[cfg(test)]"))
                .unwrap_or(false)
        })
}

#[allow(dead_code)]
pub fn workspace_members(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return vec![];
    };
    let Ok(val) = text.parse::<toml::Value>() else {
        return vec![];
    };
    val.get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn _unused_base(root: &Path) -> ProjectProfile {
    base_profile(
        root,
        Language::Rust,
        "crate",
        "cargo test",
        TestStyle::InlineCfgTest,
    )
}
