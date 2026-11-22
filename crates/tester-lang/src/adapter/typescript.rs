use super::LanguageAdapter;
use crate::index;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tester_core::model::*;

pub struct TypeScriptAdapter;

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("package.json").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let pkg_path = root.join("package.json");
        let text = std::fs::read_to_string(&pkg_path)
            .with_context(|| format!("чтение {}", pkg_path.display()))?;
        let pkg: serde_json::Value = serde_json::from_str(&text)?;

        let package_name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("package")
            .to_string();

        let deps = merge_deps(&pkg);
        let unit_framework = detect_unit_framework(root, &deps);
        let has_playwright = deps.contains_key("playwright")
            || deps.contains_key("@playwright/test")
            || root.join("playwright.config.ts").exists()
            || root.join("playwright.config.js").exists();
        let e2e_dir = ["e2e", "tests/e2e", "tests"]
            .iter()
            .map(|p| root.join(p))
            .find(|p| p.is_dir());

        let test_command = match unit_framework.as_deref() {
            Some("vitest") => "npx vitest run".into(),
            Some("jest") => "npx jest".into(),
            _ => "npm test".into(),
        };

        let test_style = if has_playwright {
            TestStyle::Playwright
        } else {
            TestStyle::NeighborFile
        };

        let notes = if has_playwright {
            vec![]
        } else {
            vec!["Playwright не установлен; перед UI-тестами выполните `tester ui init`".into()]
        };

        let mut language = Language::TypeScript;
        if !root.join("tsconfig.json").exists()
            && index::iter_source_files(root, &["ts", "tsx"]).is_empty()
        {
            language = Language::JavaScript;
        }

        Ok(ProjectProfile {
            root: root.to_path_buf(),
            language,
            package_name,
            edition: None,
            supported: true,
            test_command,
            test_style,
            src_dir: root.join("src").is_dir().then(|| root.join("src")),
            tests_dir: e2e_dir,
            has_existing_tests: has_neighbor_tests(root),
            unit_framework,
            ui_framework: has_playwright.then(|| "playwright".into()),
            ui_ready: has_playwright,
            capabilities: Capabilities {
                unit: true,
                integration: true,
                contract: true,
                ui: has_playwright,
            },
            notes,
            android_opt_in: false,
            package_filter: None,
        })
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        index::typescript::index_typescript(&profile.root)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn is_test_file(&self, path: &Path, _text: &str) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.contains(".test.")
            || name.contains(".spec.")
            || path.components().any(|c| {
                let s = c.as_os_str().to_string_lossy();
                s == "__tests__" || s == "e2e"
            })
    }

    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, kind: TestKind) -> PathBuf {
        match kind {
            TestKind::Ui => {
                if let Some(dir) = &profile.tests_dir {
                    let rel = dir.strip_prefix(&profile.root).unwrap_or(dir);
                    return rel.join(format!("{}.spec.ts", symbol.name));
                }
                PathBuf::from("e2e").join(format!("{}.spec.ts", symbol.name))
            }
            TestKind::Contract => {
                let parent = symbol.file.parent().unwrap_or(Path::new("src"));
                parent.join(format!("{}.contract.test.ts", symbol.name))
            }
            _ => {
                let parent = symbol.file.parent().unwrap_or(Path::new("src"));
                let stem = symbol
                    .file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("module");
                let neighbor = profile.root.join(parent).join(format!("{stem}.test.ts"));
                if neighbor.exists() {
                    parent.join(format!("{}.test.ts", symbol.name))
                } else {
                    parent.join(format!("{stem}.test.ts"))
                }
            }
        }
    }

    fn path_allowed(&self, profile: &ProjectProfile, kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            name,
            "package.json" | "playwright.config.ts" | "playwright.config.js"
        ) {
            return false;
        }
        match kind {
            TestKind::Ui => {
                s.starts_with("e2e/")
                    || profile
                        .tests_dir
                        .as_ref()
                        .and_then(|d| d.strip_prefix(&profile.root).ok())
                        .is_some_and(|rel| path.starts_with(rel))
            }
            _ => name.contains(".test.") || name.contains(".spec.") || s.starts_with("e2e/"),
        }
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        if draft.kind == TestKind::Ui {
            return vec![
                "npx".into(),
                "playwright".into(),
                "test".into(),
                draft.path.display().to_string(),
            ];
        }
        match profile.unit_framework.as_deref() {
            Some("jest") => vec![
                "npx".into(),
                "jest".into(),
                draft.path.display().to_string(),
                "--runInBand".into(),
            ],
            _ => vec![
                "npx".into(),
                "vitest".into(),
                "run".into(),
                draft.path.display().to_string(),
                "--pool".into(),
                "threads".into(),
                "--poolOptions.threads.singleThread".into(),
                "true".into(),
            ],
        }
    }

    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities {
        Capabilities {
            unit: true,
            integration: true,
            contract: true,
            ui: profile.ui_ready || profile.ui_framework.is_some(),
        }
    }

    fn toolchain_bin(&self, _profile: &ProjectProfile) -> &'static str {
        "npx"
    }
}

fn merge_deps(pkg: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = pkg.get(key).and_then(|v| v.as_object()) {
            for (k, v) in obj {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    out
}

fn detect_unit_framework(
    root: &Path,
    deps: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let has_vitest = deps.contains_key("vitest");
    let has_jest = deps.contains_key("jest") || deps.contains_key("@types/jest");
    let vitest_used = has_neighbor_with(root, &["vitest"]);
    let jest_used = has_neighbor_with(root, &["@jest/globals", "from \"jest\"", "from 'jest'"]);

    if vitest_used && has_vitest {
        Some("vitest".into())
    } else if jest_used && has_jest {
        Some("jest".into())
    } else if has_vitest && has_jest {
        if vitest_used {
            Some("vitest".into())
        } else if jest_used {
            Some("jest".into())
        } else {
            Some("vitest".into())
        }
    } else if has_vitest {
        Some("vitest".into())
    } else if has_jest {
        Some("jest".into())
    } else {
        None
    }
}

fn has_neighbor_tests(root: &Path) -> bool {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.path().file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.ends_with(".test.ts")
                || name.ends_with(".test.js")
                || name.ends_with(".spec.ts")
                || name.ends_with(".spec.js")
        })
        .any(|_| true)
}

fn has_neighbor_with(root: &Path, needles: &[&str]) -> bool {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.ends_with(".test.ts")
                || name.ends_with(".spec.ts")
                || name.ends_with(".test.js")
                || name.ends_with(".spec.js")
        })
        .any(|e| {
            std::fs::read_to_string(e.path())
                .map(|t| needles.iter().any(|n| t.contains(n)))
                .unwrap_or(false)
        })
}
