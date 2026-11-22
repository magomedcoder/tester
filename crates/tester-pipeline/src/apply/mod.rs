use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};
use std::path::PathBuf;
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};
use tester_core::model::{Draft, ProjectProfile};
use tester_core::security::path_escapes_root;
use tester_lang::adapter;
use tester_lang::cache;

#[derive(Debug)]
pub enum ApplyOutcome {
    Diff(String),
    Written(PathBuf),
}

pub fn apply_draft(
    profile: &ProjectProfile,
    draft: &Draft,
    write: bool,
    update: bool,
) -> Result<ApplyOutcome> {
    ensure_allowed(profile, draft)?;
    let abs = profile.root.join(&draft.path);
    if path_escapes_root(&profile.root, &draft.path) {
        return Err(AppError::PathEscapesRoot {
            path: draft.path.display().to_string(),
        }
        .into());
    }

    let old = if abs.exists() {
        std::fs::read_to_string(&abs).unwrap_or_default()
    } else {
        String::new()
    };

    let final_content = if draft.append && abs.exists() {
        merge_append(&old, &draft.content)
    } else {
        draft.content.clone()
    };

    if abs.exists() && !update && !draft.append && write {
        return Err(
            anyhow::Error::new(AppError::PathNeedsUpdate).context(draft.path.display().to_string())
        );
    }

    let diff = render_diff(&draft.path.display().to_string(), &old, &final_content);
    if !write {
        return Ok(ApplyOutcome::Diff(diff));
    }

    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).with_context(|| i18n::io_ctx(Msg::IoCreate, parent))?;
    }
    std::fs::write(&abs, &final_content).with_context(|| i18n::io_ctx(Msg::IoWrite, &abs))?;
    let _ = cache::invalidate(&profile.root);
    Ok(ApplyOutcome::Written(draft.path.clone()))
}

fn merge_append(old: &str, addition: &str) -> String {
    let add = addition.trim();
    if old.contains(add) {
        return old.to_string();
    }
    let mut out = old.trim_end().to_string();
    out.push_str("\n\n// --- дополнение tester ---\n");
    out.push_str(add);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn ensure_allowed(profile: &ProjectProfile, draft: &Draft) -> Result<()> {
    let path = &draft.path;
    let s = path.to_string_lossy();

    if s.contains("..") {
        return Err(AppError::PathForbidden {
            detail: s.to_string(),
        }
        .into());
    }

    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if matches!(
        name,
        "Cargo.toml"
            | "package.json"
            | "go.mod"
            | "pubspec.yaml"
            | "pyproject.toml"
            | "composer.json"
            | "CMakeLists.txt"
            | "build.gradle"
            | "build.gradle.kts"
            | "pom.xml"
    ) {
        return Err(AppError::ManifestWriteRefused {
            name: name.to_string(),
        }
        .into());
    }

    let adapter = adapter::adapter_for(profile.language)?;
    if !adapter.path_allowed(profile, draft.kind, path) {
        return Err(AppError::PathForbidden {
            detail: format!("{}: {}", profile.language.as_str(), s),
        }
        .into());
    }
    Ok(())
}

fn color_enabled() -> bool {
    use std::io::IsTerminal;
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn render_diff(path: &str, old: &str, new: &str) -> String {
    let color = color_enabled();
    let diff = TextDiff::from_lines(old, new);
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    for change in diff.iter_all_changes() {
        let (sign, paint) = match change.tag() {
            ChangeTag::Delete => ("-", "\x1b[31m"),
            ChangeTag::Insert => ("+", "\x1b[32m"),
            ChangeTag::Equal => (" ", ""),
        };
        if color && !paint.is_empty() {
            out.push_str(paint);
        }
        out.push_str(sign);
        out.push_str(change.value());
        if !change.value().ends_with('\n') {
            out.push('\n');
        }
        if color && !paint.is_empty() {
            out.push_str("\x1b[0m");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tester_core::model::{Capabilities, Language, TestKind, TestStyle};

    fn profile_rust(root: &Path) -> ProjectProfile {
        ProjectProfile {
            root: root.to_path_buf(),
            language: Language::Rust,
            package_name: "demo".into(),
            edition: Some("2021".into()),
            supported: true,
            test_command: "cargo test".into(),
            test_style: TestStyle::InlineCfgTest,
            src_dir: Some(root.join("src")),
            tests_dir: Some(root.join("tests")),
            has_existing_tests: false,
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
        }
    }

    #[test]
    fn refuses_lib_rs() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_rust(dir.path());
        let draft = Draft {
            path: PathBuf::from("src/lib.rs"),
            content: "fn main(){}".into(),
            kind: TestKind::Unit,
            symbol_name: "x".into(),
            filter_name: "x".into(),
            append: false,
        };
        assert!(ensure_allowed(&profile, &draft).is_err());
    }

    #[test]
    fn dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile_rust(dir.path());
        let draft = Draft {
            path: PathBuf::from("tests/foo_unit.rs"),
            content: "#[test] fn t(){ assert!(true); }\n".into(),
            kind: TestKind::Unit,
            symbol_name: "foo".into(),
            filter_name: "foo_unit".into(),
            append: false,
        };
        let out = apply_draft(&profile, &draft, false, false).unwrap();
        assert!(matches!(out, ApplyOutcome::Diff(_)));
        assert!(!dir.path().join("tests/foo_unit.rs").exists());
    }
}
