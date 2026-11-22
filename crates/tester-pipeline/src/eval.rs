use anyhow::{Result, bail};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tester_core::i18n::{self, Msg};
use tester_core::model::{Language, ProjectProfile};
use tester_lang::discover::discover;
use tester_lang::index::index_symbols;
use tester_lang::inventory::{collect_inventory, covers_symbol};

#[derive(Debug, Clone, Copy)]
pub enum MutationKind {
    Arithmetic,
    BooleanFlip,
    OffByOne,
    DeleteBranch,
    NullNone,
}

impl MutationKind {
    pub fn all() -> &'static [MutationKind] {
        &[
            Self::Arithmetic,
            Self::BooleanFlip,
            Self::OffByOne,
            Self::DeleteBranch,
            Self::NullNone,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arithmetic => "arithmetic",
            Self::BooleanFlip => "boolean_flip",
            Self::OffByOne => "off_by_one",
            Self::DeleteBranch => "delete_branch",
            Self::NullNone => "null_none",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EvalItem {
    pub symbol: String,
    pub file: PathBuf,
    pub mutation: String,
    pub caught: bool,
}

#[derive(Debug, Serialize)]
pub struct EvalReport {
    pub score: f64,
    pub items: Vec<EvalItem>,
    pub external_tool: Option<String>,
}

pub fn eval_report(path: &Path, max_symbols: usize, try_external: bool) -> Result<EvalReport> {
    let profile = discover(path)?;
    if try_external {
        if let Some(ext) = try_external_mutation(&profile) {
            return Ok(EvalReport {
                score: ext.1,
                items: vec![],
                external_tool: Some(ext.0),
            });
        }
    }

    let symbols = index_symbols(&profile)?;
    let inventory = collect_inventory(&profile)?;
    let targets: Vec<_> = symbols
        .into_iter()
        .filter(|s| covers_symbol(&inventory, &s.name) && !s.uses_io)
        .take(max_symbols.max(1))
        .collect();
    if targets.is_empty() {
        bail!("{}", i18n::t(Msg::ErrNoMutationTarget));
    }

    let mut items = Vec::new();
    for target in &targets {
        let abs = profile.root.join(&target.file);
        let original = std::fs::read_to_string(&abs)?;
        for kind in MutationKind::all() {
            let mutated = mutate_with_kind(&original, &target.body, *kind);
            if mutated == original {
                continue;
            }
            std::fs::write(&abs, &mutated)?;
            let result = Command::new("bash")
                .arg("-lc")
                .arg(&profile.test_command)
                .current_dir(&profile.root)
                .output();
            std::fs::write(&abs, &original)?;
            let caught = result.map(|o| !o.status.success()).unwrap_or(false);
            items.push(EvalItem {
                symbol: target.name.clone(),
                file: target.file.clone(),
                mutation: kind.as_str().into(),
                caught,
            });
            if caught {
                break;
            }
        }
    }

    if items.is_empty() {
        bail!("{}", i18n::t(Msg::ErrNoMutationApplied));
    }
    let caught = items.iter().filter(|i| i.caught).count() as f64;
    let score = caught / items.len() as f64;
    Ok(EvalReport {
        score,
        items,
        external_tool: None,
    })
}

pub fn eval_suite(fixtures_root: &Path) -> Result<String> {
    let mut rows = Vec::new();
    if !fixtures_root.is_dir() {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrNoFixturesDir,
                &[("path", &fixtures_root.display().to_string())],
            )
        );
    }
    for entry in std::fs::read_dir(fixtures_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        match eval_report(entry.path().as_path(), 2, false) {
            Ok(r) => rows.push(format!("| {name} | {:.2} | {} |", r.score, r.items.len())),
            Err(e) => rows.push(format!("| {name} | n/a | {e} |")),
        }
    }
    let mut out =
        String::from("# eval suite\n\n| fixture | score | items |\n| --- | --- | --- |\n");
    out.push_str(&rows.join("\n"));
    out.push('\n');
    Ok(out)
}

fn try_external_mutation(profile: &ProjectProfile) -> Option<(String, f64)> {
    let candidates: &[(&str, &[&str])] = match profile.language {
        Language::Rust => &[("cargo-mutants", &["cargo", "mutants", "--list"])],
        Language::Python => &[("mutmut", &["mutmut", "--help"])],
        Language::TypeScript | Language::JavaScript => {
            &[("stryker", &["npx", "stryker", "--help"])]
        }
        _ => &[],
    };
    for (name, argv) in candidates {
        let out = Command::new(argv[0])
            .args(&argv[1..])
            .current_dir(&profile.root)
            .output()
            .ok()?;
        if out.status.success() || name == &"mutmut" || name == &"stryker" {
            return Some((
                format!("{name} available (запустите вручную для полного отчёта)"),
                0.0,
            ));
        }
    }

    for bin in ["cargo-mutants", "mutmut", "stryker"] {
        if Command::new("bash")
            .args(["-lc", &format!("command -v {bin}")])
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some((format!("{bin} in PATH"), 0.0));
        }
    }
    let _ = profile;
    None
}

pub fn render_text(report: &EvalReport) -> String {
    let mut out = format!("score: {:.2}\n", report.score);
    if let Some(ext) = &report.external_tool {
        out.push_str(&format!("external: {ext}\n"));
    }
    for i in &report.items {
        out.push_str(&format!(
            "- {} ({}) mutation={} caught={}\n",
            i.symbol,
            i.file.display(),
            i.mutation,
            i.caught
        ));
    }
    if report.items.len() == 1 {
        out.push_str(&format!(
            "тесты_поймали_мутацию: {}\n",
            report.items[0].caught
        ));
    }
    out
}

fn mutate_with_kind(file: &str, body: &str, kind: MutationKind) -> String {
    let replacement = match kind {
        MutationKind::Arithmetic => {
            if body.contains('+') {
                body.replacen('+', "-", 1)
            } else if body.contains('*') {
                body.replacen('*', "/", 1)
            } else if body.contains('-') {
                body.replacen('-', "+", 1)
            } else {
                return file.to_string();
            }
        }
        MutationKind::BooleanFlip => {
            if body.contains("true") {
                body.replacen("true", "false", 1)
            } else if body.contains("false") {
                body.replacen("false", "true", 1)
            } else {
                return file.to_string();
            }
        }
        MutationKind::OffByOne => {
            if body.contains("1") {
                body.replacen('1', "2", 1)
            } else if body.contains("0") {
                body.replacen('0', "1", 1)
            } else {
                return file.to_string();
            }
        }
        MutationKind::DeleteBranch => {
            if body.contains(" else ") {
                body.replacen(" else ", " /*mutated*/ else ", 1)
            } else if body.contains("if ") {
                format!("{{ /* mutated branch */ {} }}", body.trim())
            } else {
                return file.to_string();
            }
        }
        MutationKind::NullNone => {
            if body.contains("Some(") {
                body.replacen("Some(", "None /*mut*/; Some(", 1)
            } else if body.contains("return ") {
                body.replacen("return ", "return None // mutated\n return ", 1)
            } else if body.contains("null") {
                body.replacen("null", "undefined", 1)
            } else {
                return file.to_string();
            }
        }
    };
    if let Some(idx) = file.find(body) {
        let mut out = file.to_string();
        out.replace_range(idx..idx + body.len(), &replacement);
        out
    } else {
        file.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_mutates_plus() {
        let file = "fn add(a:i32,b:i32)->i32{ a + b }";
        let body = "{ a + b }";
        let m = mutate_with_kind(file, body, MutationKind::Arithmetic);
        assert!(m.contains('-'), "{m}");
    }
}
