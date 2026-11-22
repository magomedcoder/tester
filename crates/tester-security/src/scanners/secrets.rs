use super::{match_rule_file, next_id, severity_ok};
use crate::model::{Finding, FixHint, PatchStrategy, ScanOptions, Severity};
use crate::rules;
use crate::walk::{self, iter_files};
use anyhow::Result;
use tester_core::model::ProjectProfile;

pub fn scan(profile: &ProjectProfile, opts: &ScanOptions) -> Result<Vec<Finding>> {
    let root = &profile.root;
    let mut out = Vec::new();
    let mut n = 0usize;

    for path in iter_files(root, opts.max_files) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for rule in rules::secret_rules() {
            match_rule_file(root, &path, &text, rule, opts, &mut n, &mut out);
        }
        if severity_ok(opts, Severity::Medium) {
            for (i, line) in text.lines().enumerate() {
                if looks_like_secret_line(line) && entropy(line) >= 4.2 {
                    n += 1;
                    out.push(Finding {
                        id: next_id("F", n),
                        rule_id: "sec.secret.entropy".into(),
                        severity: Severity::Medium,
                        category: "secrets".into(),
                        title: "High-entropy string (possible secret)".into(),
                        detail: "Review whether this is a credential; move secrets out of source."
                            .into(),
                        path: walk::rel(root, &path),
                        start_line: i + 1,
                        end_line: i + 1,
                        symbol: None,
                        snippet: Some(crate::redact::redact_snippet(line.trim())),
                        evidence: "heuristic:shannon-entropy".into(),
                        confidence: 0.45,
                        cwe: vec!["CWE-798".into()],
                        owasp: vec!["A07:2021".into()],
                        fix: FixHint {
                            summary: "Confirm and relocate secret material".into(),
                            strategy: PatchStrategy::Config,
                            template_id: None,
                        },
                    });
                }
            }
        }
    }

    let env_path = root.join(".env");
    if env_path.exists() && severity_ok(opts, Severity::High) {
        n += 1;
        out.push(Finding {
            id: next_id("F", n),
            rule_id: "sec.secret.dotenv".into(),
            severity: Severity::High,
            category: "secrets".into(),
            title: ".env file present in project tree".into(),
            detail: "Ensure .env is gitignored and never committed.".into(),
            path: walk::rel(root, &env_path),
            start_line: 1,
            end_line: 1,
            symbol: None,
            snippet: None,
            evidence: "fs:.env".into(),
            confidence: 0.9,
            cwe: vec!["CWE-260".into()],
            owasp: vec!["A07:2021".into()],
            fix: FixHint {
                summary: "Add .env to .gitignore; rotate any leaked values".into(),
                strategy: PatchStrategy::Config,
                template_id: None,
            },
        });
    }

    Ok(out)
}

fn looks_like_secret_line(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    (l.contains("key") || l.contains("secret") || l.contains("token") || l.contains("password"))
        && (line.contains('=') || line.contains(':'))
        && line.len() > 24
}

fn entropy(s: &str) -> f64 {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for b in bytes {
        freq[*b as usize] += 1;
    }
    let len = bytes.len() as f64;
    let mut h = 0.0;
    for c in freq {
        if c > 0 {
            let p = c as f64 / len;
            h -= p * p.log2();
        }
    }
    h
}
