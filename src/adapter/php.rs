use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use crate::model::*;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct PhpAdapter;

impl LanguageAdapter for PhpAdapter {
    fn language(&self) -> Language {
        Language::Php
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("composer.json").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let text =
            std::fs::read_to_string(root.join("composer.json")).unwrap_or_else(|_| "{}".into());
        let pkg: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("php_app")
            .to_string();
        let require_dev = pkg.get("require-dev").cloned().unwrap_or_default();
        let has_pest = require_dev.get("pestphp/pest").is_some();
        let has_phpunit = require_dev.get("phpunit/phpunit").is_some() || has_pest;
        let is_laravel = pkg
            .get("require")
            .and_then(|r| r.get("laravel/framework"))
            .is_some();
        let framework = if has_pest { "pest" } else { "phpunit" };
        let mut p = base_profile(
            root,
            Language::Php,
            &name,
            if has_pest {
                "./vendor/bin/pest"
            } else {
                "./vendor/bin/phpunit"
            },
            TestStyle::PhpUnit,
        );
        p.unit_framework = Some(framework.into());
        p.src_dir = root.join("src").is_dir().then(|| root.join("src"));
        p.tests_dir = root.join("tests").is_dir().then(|| root.join("tests"));
        p.has_existing_tests = p.tests_dir.is_some();
        if !root.join("vendor").is_dir() {
            p.notes
                .push("нет vendor/; перед проверкой выполните composer install".into());
        }
        if is_laravel {
            p.notes.push("обнаружен Laravel".into());
        }
        p.capabilities = Capabilities {
            unit: true,
            integration: true,
            contract: is_laravel || has_phpunit,
            ui: false,
        };
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^\s*(?:public\s+)?(?:static\s+)?function\s+(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\s*(?P<sig>\([^)]*\))",
        )?;
        let mut out = Vec::new();
        let search = profile
            .src_dir
            .clone()
            .unwrap_or_else(|| profile.root.clone());
        for file in iter_source_files(&search, &["php"]) {
            if file
                .components()
                .any(|c| matches!(c.as_os_str().to_str(), Some("vendor" | "tests" | "Tests")))
            {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if n.starts_with('_') || n == "__construct" {
                    continue;
                }
                let line_start = text[..caps.get(0).unwrap().start()]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let header = &text[line_start..caps.get(0).unwrap().start()];
                if header.contains("private ") || header.contains("protected ") {
                    continue;
                }
                let sig = caps.name("sig").unwrap().as_str();
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = extract_brace(&text, caps.get(0).unwrap().end());
                let uses_io = body.contains("file_get_contents")
                    || body.contains("fopen")
                    || body.contains("curl_")
                    || body.contains("PDO");
                out.push(Symbol {
                    name: n.into(),
                    kind: "function".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("function {n}{sig}"),
                    body,
                    is_public: true,
                    uses_io,
                    branch_score: 1,
                });
            }
        }
        Ok(out)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn is_test_file(&self, path: &Path, text: &str) -> bool {
        path.components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("tests" | "Tests")))
            || path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with("Test.php"))
            || text.contains("extends TestCase")
            || text.contains("PHPUnit")
    }

    fn suggest_path(&self, _profile: &ProjectProfile, symbol: &Symbol, _kind: TestKind) -> PathBuf {
        PathBuf::from("tests").join(format!("{}Test.php", capitalize(&symbol.name)))
    }

    fn path_allowed(&self, _profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.starts_with("tests/") && !s.contains("vendor/")
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let filter = draft
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&draft.filter_name);
        if profile.unit_framework.as_deref() == Some("pest") {
            vec!["./vendor/bin/pest".into(), draft.path.display().to_string()]
        } else {
            vec![
                "./vendor/bin/phpunit".into(),
                "--filter".into(),
                filter.into(),
            ]
        }
    }

    fn capabilities(&self, profile: &ProjectProfile) -> Capabilities {
        profile.capabilities.clone()
    }

    fn toolchain_bin(&self, _profile: &ProjectProfile) -> &'static str {
        "php"
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn extract_brace(text: &str, from: usize) -> String {
    let rest = &text[from..];
    let Some(i) = rest.find('{') else {
        return String::new();
    };
    let mut depth = 0usize;
    let bytes = rest.as_bytes();
    for j in i..bytes.len() {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return rest[i..=j].to_string();
                }
            }
            _ => {}
        }
    }
    String::new()
}
