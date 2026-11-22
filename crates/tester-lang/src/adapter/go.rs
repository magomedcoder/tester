use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use tester_core::model::*;

pub struct GoAdapter;

impl LanguageAdapter for GoAdapter {
    fn language(&self) -> Language {
        Language::Go
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("go.mod").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let mod_text = std::fs::read_to_string(root.join("go.mod")).unwrap_or_default();
        let module = mod_text
            .lines()
            .find_map(|l| l.strip_prefix("module ").map(|s| s.trim().to_string()))
            .unwrap_or_else(|| "module".into());
        let package_name = module.rsplit('/').next().unwrap_or("module").to_string();
        let has_tests = iter_source_files(root, &["go"]).iter().any(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with("_test.go"))
        });
        let mut p = base_profile(
            root,
            Language::Go,
            &package_name,
            "go test",
            TestStyle::GoTestFile,
        );
        p.unit_framework = Some("testing".into());
        p.has_existing_tests = has_tests;
        p.src_dir = Some(root.to_path_buf());
        p.capabilities = self.capabilities(&p);
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^func\s+(?:\([^)]+\)\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^)]*\))",
        )?;
        let mut out = Vec::new();
        for file in iter_source_files(&profile.root, &["go"]) {
            let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with("_test.go") {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if n == "init" {
                    continue;
                }
                let sig = caps.name("sig").unwrap().as_str();
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = extract_brace(&text, caps.get(0).unwrap().end());
                let uses_io = n == "main"
                    || body.contains("os.")
                    || body.contains("http.")
                    || body.contains("ioutil")
                    || body.contains("net.")
                    || body.contains("log.")
                    || sig.contains("http.");
                let branch_score = count_branches(&body);
                out.push(Symbol {
                    name: n.into(),
                    kind: "func".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("func {n}{sig}"),
                    body,
                    is_public: n.starts_with(|c: char| c.is_ascii_uppercase()),
                    uses_io,
                    branch_score,
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
        &["go"]
    }

    fn is_test_file(&self, path: &Path, _text: &str) -> bool {
        path.file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.ends_with("_test.go"))
    }

    fn suggest_path(&self, _profile: &ProjectProfile, symbol: &Symbol, _kind: TestKind) -> PathBuf {
        let parent = symbol.file.parent().unwrap_or(Path::new("."));
        let stem = symbol
            .file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pkg");
        parent.join(format!("{stem}_test.go"))
    }

    fn path_allowed(&self, _profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        path.file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|n| n.ends_with("_test.go"))
    }

    fn validate_command(&self, _profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let dir = draft
            .path
            .parent()
            .map(|p| {
                if p.as_os_str().is_empty() {
                    "./...".into()
                } else {
                    format!("./{}", p.display())
                }
            })
            .unwrap_or_else(|| "./...".into());
        vec![
            "go".into(),
            "test".into(),
            dir,
            "-run".into(),
            draft.filter_name.clone(),
            "-count=1".into(),
        ]
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
        "go"
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
                depth -= 1;
                if depth == 0 {
                    return rest[i..=j].to_string();
                }
            }
            _ => {}
        }
    }
    String::new()
}

fn count_branches(body: &str) -> u32 {
    [" if ", "\nif ", " for ", " switch ", " select "]
        .iter()
        .map(|k| body.matches(k).count() as u32)
        .sum::<u32>()
        .max(1)
}
