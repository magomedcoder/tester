use crate::prompts::{repair_prompt_classified, system_prompt, user_prompt};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::path::{Path, PathBuf};
use tester_core::config::Config;
use tester_core::error::AppError;
use tester_core::failure::ErrorClass;
use tester_core::gate;
use tester_core::i18n::{self, Msg};
use tester_core::model::{Draft, Language, PlanItem, ProjectProfile, TestKind};
use tester_lang::index::iter_source_files;
use tester_providers::Llm;
use tracing::debug;

pub fn generate_draft(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    config: &Config,
    item: &PlanItem,
) -> Result<Draft> {
    let style = find_style_sample(profile)?;
    let page = if item.kind == TestKind::Ui {
        read_page_context(profile)?
    } else {
        None
    };

    let system = system_prompt(item.kind);
    let user = user_prompt(profile, item, style.as_deref(), page.as_deref());
    if user.len() > config.run.max_context_chars {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrContextTooLarge,
                &[
                    ("symbol", &item.gap.symbol.name),
                    ("max", &config.run.max_context_chars.to_string()),
                ],
            )
        );
    }

    debug!(prompt = %user, "промпт генерации");
    let raw = llm.complete(&system, &user)?;
    let draft = parse_draft(profile, item, &raw)?;
    gate::check_draft(&draft)?;
    Ok(draft)
}

pub fn generate_repair(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    item: &PlanItem,
    previous: &Draft,
    error: &str,
) -> Result<Draft> {
    generate_repair_classified(
        llm,
        profile,
        item,
        previous,
        error,
        tester_core::failure::classify_error(error),
    )
}

pub fn generate_repair_classified(
    llm: &dyn Llm,
    profile: &ProjectProfile,
    item: &PlanItem,
    previous: &Draft,
    error: &str,
    class: ErrorClass,
) -> Result<Draft> {
    let system = system_prompt(item.kind);
    let user = repair_prompt_classified(
        &previous.content,
        &trim_error(error),
        &previous.path.display().to_string(),
        class,
    );
    let raw = llm.complete(&system, &user)?;
    let draft = parse_draft(profile, item, &raw)?;
    gate::check_draft(&draft)?;
    Ok(draft)
}

pub(crate) fn parse_draft(profile: &ProjectProfile, item: &PlanItem, raw: &str) -> Result<Draft> {
    let path_re = Regex::new(r"(?s)```path\s*\n\s*([^\n]+)\n\s*```")?;
    let code_re = Regex::new(
        r"(?s)```(?:rust|ts|tsx|js|javascript|typescript|go|python|py|dart|kotlin|kt|php|cpp|c\+\+|text)\s*\n(.*?)```",
    )?;

    let path = path_re
        .captures(raw)
        .map(|c| c.get(1).unwrap().as_str().trim().to_string())
        .unwrap_or_else(|| item.target_path.display().to_string());

    let content = code_re
        .captures(raw)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .or_else(|| {
            if raw.contains("#[test]") || raw.contains("test(") || raw.contains("it(") {
                Some(raw.trim().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!(i18n::t(Msg::ErrParseDraftFromLlm)))?;

    let path = PathBuf::from(path.trim_start_matches("/"));
    validate_draft_path(profile, item.kind, &path)?;

    let filter_name = infer_filter(profile, item, &path, &content);

    let draft = Draft {
        path,
        content,
        kind: item.kind,
        symbol_name: item.gap.symbol.name.clone(),
        filter_name,
        append: item.append_to_existing,
    };

    if !gate::has_assertion(&draft.content) {
        return Err(AppError::GateNoAssertion.into());
    }
    Ok(draft)
}

fn validate_draft_path(profile: &ProjectProfile, kind: TestKind, path: &Path) -> Result<()> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::PathEscapesRoot {
            path: path.display().to_string(),
        }
        .into());
    }
    let adapter = tester_lang::adapter::adapter_for(profile.language)?;
    if !adapter.path_allowed(profile, kind, path) {
        return Err(AppError::PathForbidden {
            detail: format!("{}: {}", profile.language.as_str(), path.display()),
        }
        .into());
    }
    Ok(())
}

fn infer_filter(profile: &ProjectProfile, item: &PlanItem, path: &Path, content: &str) -> String {
    match profile.language {
        Language::Rust => {
            if path.starts_with("tests") {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&item.gap.symbol.name)
                    .to_string()
            } else {
                Regex::new(r"fn\s+(test_[A-Za-z0-9_]+|[A-Za-z0-9_]+)\s*\(")
                    .ok()
                    .and_then(|re| {
                        re.captures_iter(content).find_map(|c| {
                            let n = c.get(1).unwrap().as_str();
                            if n != "main" {
                                Some(n.to_string())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_else(|| item.gap.symbol.name.clone())
            }
        }
        Language::TypeScript => path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&item.gap.symbol.name)
            .to_string(),
        _ => item.gap.symbol.name.clone(),
    }
}

fn find_style_sample(profile: &ProjectProfile) -> Result<Option<String>> {
    let exts: &[&str] = match profile.language {
        Language::Rust => &["rs"],
        Language::TypeScript => &["ts", "js"],
        _ => return Ok(None),
    };
    for file in iter_source_files(&profile.root, exts) {
        let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let is_test = match profile.language {
            Language::Rust => text.contains("#[test]"),
            Language::TypeScript => name.contains(".test.") || name.contains(".spec."),
            _ => false,
        };
        if is_test {
            let sample: String = text.chars().take(2000).collect();
            return Ok(Some(sample));
        }
    }
    Ok(None)
}

fn read_page_context(profile: &ProjectProfile) -> Result<Option<String>> {
    let candidates = ["index.html", "public/index.html", "src/index.html"];
    for c in candidates {
        let p = profile.root.join(c);
        if p.exists() {
            let text =
                std::fs::read_to_string(&p).with_context(|| i18n::io_ctx(Msg::IoRead, &p))?;
            let summary = summarize_dom(&text);
            let body: String = text.chars().take(3500).collect();
            return Ok(Some(format!("{summary}\n\nhtml:\n{body}")));
        }
    }
    Ok(None)
}

fn summarize_dom(html: &str) -> String {
    let mut bits = Vec::new();
    for (label, pat) in [
        ("roles", r#"(?i)role\s*=\s*"([^"]+)""#),
        ("labels", r"(?i)<label[^>]*>([^<]{1,40})"),
        ("buttons", r"(?i)<button[^>]*>([^<]{1,40})"),
        ("inputs", r#"(?i)<input[^>]*(?:id|name)\s*=\s*"([^"]+)""#),
        ("links", r#"(?i)<a[^>]+href\s*=\s*"([^"]+)""#),
    ] {
        if let Ok(re) = Regex::new(pat) {
            let vals: Vec<_> = re
                .captures_iter(html)
                .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
                .take(8)
                .collect();
            if !vals.is_empty() {
                bits.push(format!("{label}: {}", vals.join(", ")));
            }
        }
    }
    if bits.is_empty() {
        "dom_summary: (мало семантики - опирайся на видимый текст)".into()
    } else {
        format!("dom_summary:\n{}", bits.join("\n"))
    }
}

fn trim_error(error: &str) -> String {
    let lines: Vec<&str> = error.lines().collect();
    let interesting: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            l.contains("error")
                || l.contains("FAILED")
                || l.contains("assert")
                || l.contains("panic")
                || l.contains("Error:")
                || l.contains("Expected")
                || l.contains("Timeout")
                || l.contains("locator")
        })
        .take(80)
        .collect();
    if interesting.is_empty() {
        lines.into_iter().take(60).collect::<Vec<_>>().join("\n")
    } else {
        interesting.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tester_core::model::{Gap, PlannedCase, Symbol, TestKind};

    fn item(kind: TestKind, path: &str) -> PlanItem {
        PlanItem {
            gap: Gap {
                symbol: Symbol {
                    name: "multiply".into(),
                    kind: "fn".into(),
                    file: PathBuf::from("src/lib.rs"),
                    start_line: 1,
                    end_line: 3,
                    signature: "pub fn multiply(a: i32, b: i32) -> i32".into(),
                    body: "{ a * b }".into(),
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: String::new(),
                    visibility: String::new(),
                    parent: None,
                    docstring: String::new(),
                    start_byte: 0,
                },
                kind,
                reason: "test".into(),
                priority: 1,
            },
            kind,
            target_path: PathBuf::from(path),
            cases: vec![PlannedCase {
                name: "t".into(),
                description: "d".into(),
                input: "i".into(),
                expectation: "e".into(),
            }],
            append_to_existing: false,
            bug_context: None,
        }
    }

    fn rust_profile() -> ProjectProfile {
        ProjectProfile {
            root: PathBuf::from("."),
            language: Language::Rust,
            package_name: "demo".into(),
            edition: None,
            supported: true,
            test_command: "cargo test".into(),
            test_style: tester_core::model::TestStyle::InlineCfgTest,
            src_dir: None,
            tests_dir: None,
            has_existing_tests: false,
            unit_framework: None,
            ui_framework: None,
            ui_ready: false,
            capabilities: tester_core::model::Capabilities {
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
    fn parses_path_and_rejects_lib_rs() {
        let profile = rust_profile();
        let raw = "```path\nsrc/lib.rs\n```\n```rust\n#[test] fn t(){ assert_eq!(1,1); }\n```";
        let err = parse_draft(&profile, &item(TestKind::Unit, "tests/x.rs"), raw).unwrap_err();
        let s = err.to_string().to_ascii_lowercase();
        assert!(
            s.contains("forbidden")
                || s.contains("запрещён")
                || s.contains("policy")
                || s.contains("tests/")
        );
    }

    #[test]
    fn parses_valid_unit_draft() {
        let profile = rust_profile();
        let raw = "```path\ntests/multiply_unit.rs\n```\n```rust\n#[test] fn multiply_happy(){ assert_eq!(1,1); }\n```";
        let draft = parse_draft(
            &profile,
            &item(TestKind::Unit, "tests/multiply_unit.rs"),
            raw,
        )
        .unwrap();
        assert_eq!(draft.path, PathBuf::from("tests/multiply_unit.rs"));
    }
}
