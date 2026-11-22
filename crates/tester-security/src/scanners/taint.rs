use super::{next_id, severity_ok};
use crate::model::{Finding, FixHint, PatchStrategy, ScanOptions, Severity};
use crate::walk::{self, iter_files};
use anyhow::Result;
use regex::Regex;
use tester_core::model::ProjectProfile;

const SOURCES: &[&str] = &[
    r"request\.(args|form|json|data|GET|POST)",
    r"req\.(body|query|params)",
    r"std::env::args",
    r"os\.environ",
    r"process\.argv",
    r"\$_(GET|POST|REQUEST|COOKIE)",
    r"gin\.Context|c\.Query\(|c\.Param\(",
    r"HttpServletRequest|@RequestParam|@PathVariable",
];

const SINKS: &[&str] = &[
    r"os\.system\s*\(",
    r"subprocess\.(call|Popen|run)\s*\(",
    r"eval\s*\(",
    r"exec\s*\(",
    r"pickle\.loads\s*\(",
    r"child_process\.(exec|execSync)",
    r"shell_exec\s*\(",
    r"Command::new\s*\(",
    r"exec\.Command\s*\(",
];

pub fn scan(
    profile: &ProjectProfile,
    opts: &ScanOptions,
    existing: &[Finding],
) -> Result<Vec<Finding>> {
    let root = &profile.root;
    let source_res: Vec<Regex> = SOURCES.iter().filter_map(|p| Regex::new(p).ok()).collect();
    let sink_res: Vec<Regex> = SINKS.iter().filter_map(|p| Regex::new(p).ok()).collect();
    let mut out = Vec::new();
    let mut n = existing.len();

    for path in iter_files(root, opts.max_files.min(1500)) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let has_source = source_res.iter().any(|r| r.is_match(&text));
        let sink_hit = sink_res.iter().find_map(|r| r.find(&text));
        if !(has_source && sink_hit.is_some()) {
            continue;
        }
        if !severity_ok(opts, Severity::Critical) {
            continue;
        }
        let m = sink_hit.unwrap();
        let line = text[..m.start()].bytes().filter(|b| *b == b'\n').count() + 1;
        n += 1;
        out.push(Finding {
            id: next_id("F", n),
            rule_id: "sec.taint.source_to_sink".into(),
            severity: Severity::Critical,
            category: "rce".into(),
            title: "Potential untrusted data reachability to dangerous sink".into(),
            detail: "Same file contains request/env/argv-like sources and an execution/deserialization sink. Harden the sink and validate inputs.".into(),
            path: walk::rel(root, &path),
            start_line: line,
            end_line: line,
            symbol: None,
            snippet: Some(crate::redact::redact_snippet(
                text.lines().nth(line - 1).unwrap_or("").trim(),
            )),
            evidence: "taint:intra-file".into(),
            confidence: 0.55,
            cwe: vec!["CWE-20".into(), "CWE-78".into()],
            owasp: vec!["A03:2021".into()],
            fix: FixHint {
                summary: "Validate/sanitize sources; replace sink with safe API".into(),
                strategy: PatchStrategy::Sanitize,
                template_id: None,
            },
        });
    }

    let _ = profile;
    Ok(out)
}
