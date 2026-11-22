use crate::model::{Finding, PatchDraft, SecurityReport};
use crate::redact::redact_snippet;
use anyhow::{Context, Result, bail};
use std::path::Path;
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};
use tester_core::model::ProjectProfile;
use tester_core::security::path_escapes_root;
use tester_llm::Llm;

pub fn explain(report: &SecurityReport, finding_id: &str) -> Result<String> {
    let f = report
        .find(finding_id)
        .ok_or_else(|| AppError::FindingNotFound {
            id: finding_id.to_string(),
        })?;
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", f.title));
    out.push_str(&format!(
        "- id: {}\n- rule: {}\n- severity: {}\n- file: {}:{}\n- evidence: {}\n- confidence: {:.2}\n",
        f.id,
        f.rule_id,
        f.severity.as_str(),
        f.path.display(),
        f.start_line,
        f.evidence,
        f.confidence
    ));
    if let Some(s) = &f.snippet {
        out.push_str(&format!("\nSnippet:\n  {}\n", redact_snippet(s)));
    }
    out.push_str(&format!("\n{}\n\n", f.detail));
    out.push_str(&format!(
        "Remediation ({}) : {}\n",
        f.fix.strategy.as_str(),
        f.fix.summary
    ));
    out.push_str(
        "\nPolicy: defensive harden only - do not generate exploit steps or attack PoCs.\n",
    );
    Ok(out)
}

pub fn draft_patch(
    profile: &ProjectProfile,
    finding: &Finding,
    llm: Option<&dyn Llm>,
    apply: bool,
) -> Result<PatchDraft> {
    if path_escapes_root(&profile.root, &finding.path) {
        return Err(AppError::PathEscapesRoot {
            path: finding.path.display().to_string(),
        }
        .into());
    }
    let abs = profile.root.join(&finding.path);
    let original =
        std::fs::read_to_string(&abs).with_context(|| i18n::io_ctx(Msg::IoRead, &abs))?;

    let (new_text, note) = if let Some(tid) = finding.fix.template_id.as_deref() {
        if let Some(t) = apply_template(tid, &original) {
            (t, format!("template:{tid}"))
        } else if let Some(llm) = llm {
            llm_patch(llm, finding, &original)?
        } else {
            bail!("no template edit for {tid}; pass LLM for draft or choose another finding");
        }
    } else if let Some(llm) = llm {
        llm_patch(llm, finding, &original)?
    } else {
        let advice = format!(
            "{}\n\n# SECURITY HARDEN HINT (not applied)\n# rule: {}\n# {}\n",
            original.trim_end(),
            finding.rule_id,
            finding.fix.summary
        );
        (advice, "advise-only (no template / no llm)".into())
    };

    let diff = unified_diff(&finding.path.display().to_string(), &original, &new_text);
    let mut draft = PatchDraft {
        finding_id: finding.id.clone(),
        path: finding.path.clone(),
        dry_run: !apply,
        applied: false,
        diff,
        note,
    };

    if apply {
        if new_text == original {
            bail!("patch produced no changes");
        }
        if draft.note.starts_with("advise-only") {
            bail!("refusing to apply advise-only patch; use --dry-run or enable LLM/template");
        }
        std::fs::write(&abs, &new_text).with_context(|| i18n::io_ctx(Msg::IoWrite, &abs))?;
        draft.applied = true;
        draft.dry_run = false;
        append_audit(&profile.root, finding, &draft)?;
    }
    Ok(draft)
}

fn apply_template(id: &str, original: &str) -> Option<String> {
    match id {
        "py_yaml_load" => {
            let mut t = original.replace("yaml.load(", "yaml.safe_load(");
            if let Ok(re) = regex::Regex::new(r"yaml\.safe_load\(([^)]*),\s*Loader\s*=\s*[^)]+\)") {
                t = re.replace_all(&t, "yaml.safe_load($1)").into_owned();
            }
            Some(t)
        }
        "py_eval" => {
            if original.contains("eval(") {
                Some(original.replacen("eval(", "ast.literal_eval(", 1))
            } else {
                None
            }
        }
        "py_os_system" => {
            let re = regex::Regex::new(r"os\.system\s*\(([^)]*)\)").ok()?;
            let mut t = re
                .replace_all(original, "subprocess.run($1, shell=False)")
                .into_owned();
            if t.contains("subprocess.run") && !t.contains("import subprocess") {
                t = format!("import subprocess\n{t}");
            }
            Some(t)
        }
        _ => None,
    }
}

fn llm_patch(llm: &dyn Llm, finding: &Finding, original: &str) -> Result<(String, String)> {
    let system = r#"You are a defensive security engineer.
Return ONLY the full updated file contents after a minimal harden fix.
Do NOT explain exploits or attack steps.
Do NOT add markdown fences.
Prefer small safe API changes (safe_load, subprocess without shell, remove secrets)."#;
    let user = format!(
        "Finding: {} ({})\nSeverity: {}\nFix hint: {}\nDetail: {}\n\nFILE:\n{}",
        finding.title,
        finding.rule_id,
        finding.severity.as_str(),
        finding.fix.summary,
        finding.detail,
        original
    );
    let raw = llm.complete(system, &user)?;
    let text = strip_fences(&raw);
    if text.trim().is_empty() || text.trim() == original.trim() {
        bail!("llm produced empty/unchanged patch");
    }
    Ok((text, "llm-draft".into()))
}

fn strip_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let rest = rest.find('\n').map(|i| &rest[i + 1..]).unwrap_or(rest);
        if let Some(end) = rest.rfind("```") {
            return rest[..end].to_string();
        }
    }
    s.to_string()
}

fn unified_diff(path: &str, old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    let max = old_lines.len().max(new_lines.len());
    let mut start = 0;
    while start < max && old_lines.get(start) == new_lines.get(start) {
        start += 1;
    }
    let mut end_old = old_lines.len();
    let mut end_new = new_lines.len();
    while end_old > start
        && end_new > start
        && old_lines.get(end_old - 1) == new_lines.get(end_new - 1)
    {
        end_old -= 1;
        end_new -= 1;
    }
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        start + 1,
        end_old.saturating_sub(start),
        start + 1,
        end_new.saturating_sub(start)
    ));
    for l in &old_lines[start..end_old] {
        out.push_str(&format!("-{l}\n"));
    }
    for l in &new_lines[start..end_new] {
        out.push_str(&format!("+{l}\n"));
    }
    out
}

fn append_audit(root: &Path, finding: &Finding, draft: &PatchDraft) -> Result<()> {
    let dir = root.join(".tester");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("audit.jsonl");
    let line = serde_json::json!({
        "ts": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "event": "security_patch",
        "finding_id": finding.id,
        "rule_id": finding.rule_id,
        "path": draft.path,
        "applied": draft.applied,
        "note": draft.note,
    });
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_yaml_safe_load() {
        let src = "data = yaml.load(stream)\n";
        let out = apply_template("py_yaml_load", src).unwrap();
        assert!(out.contains("safe_load"));
        assert!(!out.contains("yaml.load("));
    }

    #[test]
    fn template_os_system() {
        let src = "import os\nos.system(cmd)\n";
        let out = apply_template("py_os_system", src).unwrap();
        assert!(out.contains("subprocess.run"));
        assert!(out.contains("shell=False"));
    }
}
