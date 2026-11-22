use super::{next_id, severity_ok};
use crate::model::{Finding, FixHint, PatchStrategy, ScanOptions, Severity};
use crate::walk;
use anyhow::Result;
use tester_core::model::ProjectProfile;

pub fn scan(
    profile: &ProjectProfile,
    opts: &ScanOptions,
    notes: &mut Vec<String>,
) -> Result<Vec<Finding>> {
    let root = &profile.root;
    let mut out = Vec::new();
    let mut n = 0usize;

    let lockfiles = [
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "go.sum",
        "composer.lock",
        "uv.lock",
        "requirements.txt",
    ];
    let mut found_any = false;
    for name in lockfiles {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        found_any = true;
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if name == "requirements.txt" {
            for (i, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                if line.contains("==") && severity_ok(opts, Severity::Info) {
                    n += 1;
                    out.push(Finding {
                        id: next_id("F", n),
                        rule_id: "sec.deps.pinned_review".into(),
                        severity: Severity::Info,
                        category: "deps".into(),
                        title: format!("Pinned dependency listed: {line}"),
                        detail: "Run with --fetch-advisories (or config fetch_advisories) for CVE matching.".into(),
                        path: walk::rel(root, &path),
                        start_line: i + 1,
                        end_line: i + 1,
                        symbol: None,
                        snippet: Some(line.chars().take(120).collect()),
                        evidence: "lockfile:requirements".into(),
                        confidence: 0.3,
                        cwe: vec![],
                        owasp: vec!["A06:2021".into()],
                        fix: FixHint {
                            summary: "Keep dependencies updated; check OSV/GitHub Advisories".into(),
                            strategy: PatchStrategy::Deps,
                            template_id: None,
                        },
                    });
                }
            }
        }
    }

    if !found_any {
        notes.push("no lockfile found for dependency advisory pass".into());
    } else if opts.fetch_advisories {
        match crate::advisory::query_osv_stub(root) {
            Ok(extra) => {
                notes.push(format!("advisory fetch: {} hits", extra.len()));
                for mut f in extra {
                    n += 1;
                    f.id = next_id("F", n);
                    if severity_ok(opts, f.severity) {
                        out.push(f);
                    }
                }
            }
            Err(e) => notes.push(format!("advisory fetch skipped: {e}")),
        }
    } else {
        notes.push("dependency advisories: offline stub (enable fetch_advisories for OSV)".into());
    }

    Ok(out)
}
