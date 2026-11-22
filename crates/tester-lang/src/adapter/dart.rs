use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use tester_core::model::*;

pub struct DartAdapter;

impl LanguageAdapter for DartAdapter {
    fn language(&self) -> Language {
        Language::Dart
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("pubspec.yaml").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let text = std::fs::read_to_string(root.join("pubspec.yaml")).unwrap_or_default();
        let name = text
            .lines()
            .find_map(|l| l.strip_prefix("name:").map(|s| s.trim().to_string()))
            .unwrap_or_else(|| "dart_app".into());
        let is_flutter = text.contains("flutter:") || text.contains("sdk: flutter");
        let lang = if is_flutter {
            Language::Flutter
        } else {
            Language::Dart
        };
        let has_integration = text.contains("integration_test");
        let cmd = if is_flutter {
            "flutter test"
        } else {
            "dart test"
        };
        let mut p = base_profile(root, lang, &name, cmd, TestStyle::DartTest);
        p.unit_framework = Some(if is_flutter {
            "flutter_test".into()
        } else {
            "test".into()
        });
        p.src_dir = root.join("lib").is_dir().then(|| root.join("lib"));
        p.tests_dir = root.join("test").is_dir().then(|| root.join("test"));
        p.has_existing_tests = root.join("test").is_dir();
        p.ui_framework = is_flutter.then(|| "flutter_test".into());
        p.ui_ready = is_flutter;
        if is_flutter && !has_integration {
            p.notes
                .push("integration_test нет в pubspec; plan предупредит для integration".into());
        }
        p.capabilities = Capabilities {
            unit: true,
            integration: has_integration || !is_flutter,
            contract: false,
            ui: is_flutter,
        };
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^(?P<indent>\s*)(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\s*(?P<sig>\([^;{]*\))\s*(?:async\s*)?\{",
        )?;
        let mut out = Vec::new();
        let root_lib = profile.root.join("lib");
        let search = if root_lib.is_dir() {
            root_lib
        } else {
            profile.root.clone()
        };
        for file in iter_source_files(&search, &["dart"]) {
            if file
                .components()
                .any(|c| c.as_os_str() == "test" || c.as_os_str() == "integration_test")
            {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if n.starts_with('_') || n == "main" || n == "build" || n == "createState" {
                    continue;
                }

                let sig = caps.name("sig").unwrap().as_str();
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = "{ ... }".into();
                out.push(Symbol {
                    name: n.into(),
                    kind: "fn".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line,
                    signature: format!("{n}{sig}"),
                    body,
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: String::new(),
                    visibility: String::new(),
                    parent: None,
                    docstring: String::new(),
                    start_byte: 0,
                });
            }
        }
        Ok(out)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["dart"]
    }

    fn is_test_file(&self, path: &Path, _text: &str) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.ends_with("_test.dart")
            || path
                .components()
                .any(|c| matches!(c.as_os_str().to_str(), Some("test" | "integration_test")))
    }

    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf {
        match kind {
            TestKind::Ui | TestKind::Integration if profile.language == Language::Flutter => {
                if profile
                    .notes
                    .iter()
                    .any(|n| n.contains("integration_test not"))
                    && kind == TestKind::Integration
                {
                    PathBuf::from("test").join(format!("{}_test.dart", symbol.name))
                } else if kind == TestKind::Integration {
                    PathBuf::from("integration_test").join(format!("{}_test.dart", symbol.name))
                } else {
                    PathBuf::from("test").join(format!("{}_widget_test.dart", symbol.name))
                }
            }
            _ => PathBuf::from("test").join(format!("{}_test.dart", symbol.name)),
        }
    }

    fn path_allowed(&self, _profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        (s.starts_with("test/") || s.starts_with("integration_test/")) && !s.starts_with("lib/")
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        if profile.language == Language::Flutter {
            vec![
                "flutter".into(),
                "test".into(),
                draft.path.display().to_string(),
            ]
        } else {
            vec![
                "dart".into(),
                "test".into(),
                draft.path.display().to_string(),
            ]
        }
    }

    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities {
        profile.capabilities.clone()
    }

    fn toolchain_bin(&self, profile: &ProjectProfile) -> &'static str {
        if profile.language == Language::Flutter {
            "flutter"
        } else {
            "dart"
        }
    }
}
