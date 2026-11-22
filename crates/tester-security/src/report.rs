use crate::model::{SecurityReport, Severity};
use crate::redact::redact_snippet;
use tester_core::i18n::{self, Msg};

pub fn render_json(report: &SecurityReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".into())
}

pub fn render_markdown(report: &SecurityReport) -> String {
    let mut out = String::new();
    out.push_str(i18n::t(Msg::SecReportTitle));
    out.push_str("\n\n");
    out.push_str(&i18n::tr(
        Msg::SecMetaLine,
        &[
            ("root", &report.root.display().to_string()),
            ("pack", &report.rule_pack_version),
            ("deep", &report.deep.to_string()),
            ("n", &report.findings.len().to_string()),
        ],
    ));
    out.push_str(&format!("\n{}\n\n", i18n::t(Msg::SecCounts)));
    for (sev, n) in report.counts_by_severity() {
        if n > 0 {
            out.push_str(&format!("- {}: {n}\n", sev.as_str()));
        }
    }
    if !report.notes.is_empty() {
        out.push_str(&format!("\n{}\n\n", i18n::t(Msg::SecNotes)));
        for n in &report.notes {
            out.push_str(&format!("- {n}\n"));
        }
    }
    out.push_str(&format!("\n{}\n\n", i18n::t(Msg::SecFindings)));
    if report.findings.is_empty() {
        out.push_str(i18n::t(Msg::SecNoFindings));
        out.push('\n');
        return out;
    }
    for f in &report.findings {
        out.push_str(&format!(
            "### {} {} `{}`\n\n",
            f.id,
            f.severity.as_str(),
            f.rule_id
        ));
        out.push_str(&format!("**{}**\n\n", f.title));
        out.push_str(&i18n::tr(
            Msg::SecFindingMeta,
            &[
                ("path", &f.path.display().to_string()),
                ("line", &f.start_line.to_string()),
                ("category", &f.category.to_string()),
                ("confidence", &format!("{:.2}", f.confidence)),
                ("evidence", &f.evidence.to_string()),
            ],
        ));
        if !f.cwe.is_empty() {
            out.push_str(&format!("- cwe: {}\n", f.cwe.join(", ")));
        }
        if let Some(snip) = &f.snippet {
            out.push_str(&format!("\n```\n{}\n```\n", redact_snippet(snip)));
        }
        out.push('\n');
        out.push_str(&i18n::tr(
            Msg::SecFixLine,
            &[
                ("strategy", f.fix.strategy.as_str()),
                ("summary", &f.fix.summary),
            ],
        ));
        out.push_str("\n\n");
        out.push_str(&format!("{}\n\n", f.detail));
    }
    out
}

pub fn render_text(report: &SecurityReport) -> String {
    let mut out = String::new();
    out.push_str(&i18n::tr(
        Msg::SecScanSummary,
        &[
            ("n", &report.findings.len().to_string()),
            ("pack", "rule_pack"),
            ("version", &report.rule_pack_version),
        ],
    ));
    out.push('\n');
    for (sev, n) in report.counts_by_severity() {
        if n > 0 {
            out.push_str(&format!("  {}: {n}\n", sev.as_str()));
        }
    }
    for f in &report.findings {
        out.push_str(&format!(
            "[{}] {} {} {}:{} - {}\n",
            f.severity.as_str(),
            f.id,
            f.rule_id,
            f.path.display(),
            f.start_line,
            f.title
        ));
    }
    for n in &report.notes {
        out.push_str(&i18n::tr(Msg::SecNoteLine, &[("note", n)]));
        out.push('\n');
    }
    out
}

pub fn exit_code(report: &SecurityReport, fail_on: Severity) -> u8 {
    if report.above(fail_on).next().is_some() {
        2
    } else {
        0
    }
}
