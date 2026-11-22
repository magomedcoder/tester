mod config_scan;
mod deps;
mod secrets;
mod sinks;
mod taint;

use crate::model::{Finding, ScanOptions};
use anyhow::Result;
use std::path::Path;
use tester_core::model::ProjectProfile;

pub fn run_all(
    profile: &ProjectProfile,
    opts: &ScanOptions,
    notes: &mut Vec<String>,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    findings.extend(secrets::scan(profile, opts)?);
    findings.extend(sinks::scan(profile, opts)?);
    findings.extend(config_scan::scan(profile, opts)?);
    findings.extend(deps::scan(profile, opts, notes)?);
    if opts.deep {
        findings.extend(taint::scan(profile, opts, &findings)?);
        notes.push("deep taint pass enabled (intra-file source->sink)".into());
    }
    Ok(findings)
}

pub(crate) fn next_id(prefix: &str, n: usize) -> String {
    format!("{prefix}-{n:04}")
}

pub(crate) fn allowed(opts: &ScanOptions, rule_id: &str) -> bool {
    !opts
        .disabled_rules
        .iter()
        .any(|d| d == rule_id || rule_id.starts_with(d.trim_end_matches('*')))
}

pub(crate) fn severity_ok(opts: &ScanOptions, sev: crate::model::Severity) -> bool {
    opts.severities.contains(&sev)
}

pub(crate) fn line_of(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx.min(text.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}

pub(crate) fn snippet_at(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .chars()
        .take(200)
        .collect()
}

pub(crate) fn match_rule_file(
    root: &Path,
    path: &Path,
    text: &str,
    rule: &crate::rules::PatternRule,
    opts: &ScanOptions,
    id_counter: &mut usize,
    out: &mut Vec<Finding>,
) {
    if !allowed(opts, rule.id) || !severity_ok(opts, rule.severity) {
        return;
    }
    if !rule.extensions.is_empty() {
        let ext = crate::walk::ext_of(path);
        if !rule.extensions.iter().any(|e| *e == ext) {
            return;
        }
    }
    let Ok(re) = regex::Regex::new(rule.pattern) else {
        return;
    };
    for m in re.find_iter(text) {
        *id_counter += 1;
        let line = line_of(text, m.start());
        let snip = crate::redact::redact_snippet(&snippet_at(text, line));
        out.push(Finding {
            id: next_id("F", *id_counter),
            rule_id: rule.id.into(),
            severity: rule.severity,
            category: rule.category.into(),
            title: rule.title.into(),
            detail: rule.detail.into(),
            path: crate::walk::rel(root, path),
            start_line: line,
            end_line: line,
            symbol: None,
            snippet: Some(snip),
            evidence: rule.evidence.into(),
            confidence: 0.75,
            cwe: rule.cwe.iter().map(|s| (*s).into()).collect(),
            owasp: rule.owasp.iter().map(|s| (*s).into()).collect(),
            fix: crate::rules::fix_hint(rule),
        });
    }
}
