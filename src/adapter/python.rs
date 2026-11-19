use super::{LanguageAdapter, base_profile};
use crate::index::iter_source_files;
use crate::model::*;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> Language {
        Language::Python
    }

    fn detect(&self, root: &Path) -> bool {
        root.join("pyproject.toml").exists()
            || root.join("requirements.txt").exists()
            || root.join("setup.cfg").exists()
    }

    fn discover(&self, root: &Path) -> Result<ProjectProfile> {
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("python_project")
            .to_string();
        let has_pytest = root.join("pytest.ini").exists()
            || std::fs::read_to_string(root.join("pyproject.toml"))
                .map(|t| t.contains("pytest"))
                .unwrap_or(false)
            || iter_source_files(root, &["py"]).iter().any(|p| {
                std::fs::read_to_string(p)
                    .map(|t| t.contains("import pytest") || t.contains("from pytest"))
                    .unwrap_or(false)
            });
        let framework = if has_pytest { "pytest" } else { "unittest" };
        let cmd = if has_pytest {
            "pytest"
        } else {
            "python -m unittest"
        };
        let mut p = base_profile(root, Language::Python, &name, cmd, TestStyle::Pytest);
        p.unit_framework = Some(framework.into());
        p.src_dir = root.join("src").is_dir().then(|| root.join("src"));
        p.tests_dir = root.join("tests").is_dir().then(|| root.join("tests"));
        p.has_existing_tests = iter_source_files(root, &["py"]).iter().any(|f| {
            let n = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("test_") || n.ends_with("_test.py")
        });
        if root.join(".venv").exists() {
            p.notes.push("найден .venv".into());
        }
        p.capabilities = self.capabilities(&p);
        Ok(p)
    }

    fn index(&self, profile: &ProjectProfile) -> Result<Vec<Symbol>> {
        let fn_re = Regex::new(
            r"(?m)^(?P<indent> *)(?P<async>async\s+)?def\s+(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\s*(?P<sig>\([^)]*\))",
        )?;
        let mut out = Vec::new();
        for file in iter_source_files(&profile.root, &["py"]) {
            let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("test_") || name.ends_with("_test.py") {
                continue;
            }
            if file.components().any(|c| {
                matches!(
                    c.as_os_str().to_str(),
                    Some(".venv" | "venv" | "site-packages" | "__pycache__")
                )
            }) {
                continue;
            }
            let text = std::fs::read_to_string(&file)?;
            let rel = file.strip_prefix(&profile.root).unwrap_or(&file);
            for caps in fn_re.captures_iter(&text) {
                let n = caps.name("name").unwrap().as_str();
                if n.starts_with('_') {
                    continue;
                }
                let indent = caps.name("indent").unwrap().as_str().len();

                if indent > 4 {
                    continue;
                }
                let sig = caps.name("sig").unwrap().as_str();
                let start = caps.get(0).unwrap().start();
                let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
                let body = extract_indent_block(&text, caps.get(0).unwrap().end(), indent);
                let uses_io = body.contains("open(")
                    || body.contains("pathlib")
                    || body.contains("requests.")
                    || body.contains("httpx")
                    || body.contains("urllib");
                let branch_score = [" if ", "\nif ", " for ", " while ", " match "]
                    .iter()
                    .map(|k| body.matches(k).count() as u32)
                    .sum::<u32>()
                    .max(1);
                out.push(Symbol {
                    name: n.into(),
                    kind: "def".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("def {n}{sig}"),
                    body,
                    is_public: true,
                    uses_io,
                    branch_score,
                });
            }
        }
        Ok(out)
    }

    fn inventory_globs(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn is_test_file(&self, path: &Path, _text: &str) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.starts_with("test_") || name.ends_with("_test.py")
    }

    fn suggest_path(&self, profile: &ProjectProfile, symbol: &Symbol, _kind: TestKind) -> PathBuf {
        if profile.tests_dir.is_some() {
            PathBuf::from("tests").join(format!("test_{}.py", symbol.name))
        } else {
            let parent = symbol.file.parent().unwrap_or(Path::new("."));
            parent.join(format!("test_{}.py", symbol.name))
        }
    }

    fn path_allowed(&self, _profile: &ProjectProfile, _kind: TestKind, path: &Path) -> bool {
        let s = path.to_string_lossy();
        if s.contains("site-packages") || s.contains(".venv") {
            return false;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.starts_with("test_") || name.ends_with("_test.py") || s.starts_with("tests/")
    }

    fn validate_command(&self, profile: &ProjectProfile, draft: &Draft) -> Vec<String> {
        let pytest = profile.root.join(".venv/bin/pytest");
        let python = profile.root.join(".venv/bin/python");
        match profile.unit_framework.as_deref() {
            Some("unittest") => {
                let py = if python.exists() {
                    python.display().to_string()
                } else {
                    "python3".into()
                };
                vec![
                    py,
                    "-m".into(),
                    "unittest".into(),
                    draft.path.display().to_string(),
                ]
            }
            _ => {
                let bin = if pytest.exists() {
                    pytest.display().to_string()
                } else {
                    "pytest".into()
                };
                vec![
                    bin,
                    format!("{}::{}", draft.path.display(), draft.filter_name),
                    "-q".into(),
                ]
            }
        }
    }

    fn capabilities(&self, _profile: &ProjectProfile) -> Capabilities {
        Capabilities {
            unit: true,
            integration: true,
            contract: true,
            ui: false,
        }
    }

    fn toolchain_bin(&self, profile: &ProjectProfile) -> &'static str {
        match profile.unit_framework.as_deref() {
            Some("unittest") => "python3",
            _ => "pytest",
        }
    }
}

fn extract_indent_block(text: &str, from: usize, base_indent: usize) -> String {
    let rest = &text[from..];
    let mut lines = rest.lines();
    let mut out = String::new();

    if let Some(line) = lines.next() {
        out.push_str(line);
        out.push('\n');
    }

    for line in lines {
        let indent = line.chars().take_while(|c| *c == ' ').count();
        if !line.trim().is_empty() && indent <= base_indent {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}
