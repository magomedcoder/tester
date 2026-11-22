use serde_json::json;
use tester_core::i18n::{self, Msg};
use tester_core::model::{ItemStatus, ProjectProfile, RunReport, TestKind};

fn status_label(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Planned => i18n::t(Msg::ReportStatusPlanned),
        ItemStatus::DryRun => i18n::t(Msg::ReportStatusDryRun),
        ItemStatus::Written => i18n::t(Msg::ReportStatusWritten),
        ItemStatus::Passed => i18n::t(Msg::ReportStatusPassed),
        ItemStatus::Unfixed => i18n::t(Msg::ReportStatusUnfixed),
        ItemStatus::Skipped => i18n::t(Msg::ReportStatusSkipped),
        ItemStatus::FailedPath => i18n::t(Msg::ReportStatusFailedPath),
    }
}

pub fn render_markdown(profile: &ProjectProfile, report: &RunReport) -> String {
    let mut out = String::new();
    out.push_str(i18n::t(Msg::ReportTitle));
    out.push_str(&format!(
        "- {}: `{}`\n",
        i18n::t(Msg::ReportProject),
        profile.root.display()
    ));
    out.push_str(&format!(
        "- {}: {}\n",
        i18n::t(Msg::ReportLanguage),
        profile.language.as_str()
    ));
    out.push_str(&format!(
        "- {}: {}\n",
        i18n::t(Msg::ReportPackage),
        profile.package_name
    ));
    if let Some(r) = report.metrics.unit_first_pass_rate {
        out.push_str(&format!("- unit_first_pass_rate: {:.0}%\n", r * 100.0));
    }
    if let Some(u) = report.metrics.usefulness_avg {
        out.push_str(&format!("- usefulness_avg: {:.2}\n", u));
    }
    out.push('\n');

    if !report.warnings.is_empty() {
        out.push_str(i18n::t(Msg::ReportWarnings));
        for w in &report.warnings {
            out.push_str(&format!("- {w}\n"));
        }
        out.push('\n');
    }

    let unit_fail = report
        .items
        .iter()
        .filter(|i| {
            i.kind == TestKind::Unit
                && matches!(i.status, ItemStatus::Unfixed | ItemStatus::FailedPath)
        })
        .count();
    let int_fail = report
        .items
        .iter()
        .filter(|i| {
            i.kind == TestKind::Integration
                && matches!(i.status, ItemStatus::Unfixed | ItemStatus::FailedPath)
        })
        .count();
    if unit_fail + int_fail > 0 {
        out.push_str(i18n::t(Msg::ReportFailures));
        out.push_str(&format!(
            "- unit: {unit_fail}\n- integration: {int_fail}\n\n"
        ));
    }

    out.push_str(i18n::t(Msg::ReportResults));
    for item in &report.items {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | `{}` |\n",
            item.symbol,
            item.kind.as_str(),
            item.file.display(),
            item.attempts,
            status_label(item.status),
            item.first_pass
                .map(|b| if b { "yes" } else { "no" })
                .unwrap_or("-"),
            item.assert_density
                .map(|d| format!("{d:.2}"))
                .unwrap_or_else(|| "-".into()),
            item.usefulness
                .map(|u| format!("{u:.2}"))
                .unwrap_or_else(|| "-".into()),
            item.command.replace('|', "\\|")
        ));
    }
    out.push('\n');

    for item in &report.items {
        if matches!(
            item.status,
            ItemStatus::DryRun | ItemStatus::Unfixed | ItemStatus::FailedPath
        ) {
            out.push_str(&format!(
                "### {} ({})\n\n",
                item.symbol,
                status_label(item.status)
            ));
            out.push_str("```\n");
            out.push_str(&item.message);
            if !item.message.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
        if let Some(trace) = &item.trace_path {
            out.push_str(&format!(
                "{}: `{}`\n\n",
                i18n::t(Msg::ReportTrace),
                trace.display()
            ));
        }
    }
    out
}

pub fn render_json(profile: &ProjectProfile, report: &RunReport) -> String {
    let value = json!({
        "schema": "tester.report.v1",
        "project": profile.root,
        "language": profile.language,
        "package": profile.package_name,
        "warnings": report.warnings,
        "metrics": report.metrics,
        "gates": {
            "max_unfixed": report.items.iter().filter(|i| i.status == ItemStatus::Unfixed).count(),
            "min_usefulness": report.metrics.usefulness_avg,
            "unit_first_pass_rate": report.metrics.unit_first_pass_rate,
        },
        "items": report.items,
        "by_kind_failures": {
            "unit": report.items.iter().filter(|i| i.kind == TestKind::Unit && i.status == ItemStatus::Unfixed).count(),
            "integration": report.items.iter().filter(|i| i.kind == TestKind::Integration && i.status == ItemStatus::Unfixed).count(),
            "ui": report.items.iter().filter(|i| i.kind == TestKind::Ui && i.status == ItemStatus::Unfixed).count(),
            "contract": report.items.iter().filter(|i| i.kind == TestKind::Contract && i.status == ItemStatus::Unfixed).count(),
        }
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

pub fn exit_for(report: &RunReport) -> i32 {
    tester_core::error::exit_from_report(report).0 as i32
}

pub fn render_usage(usage: &tester_llm::Usage) -> String {
    i18n::tr(
        Msg::UsageLine,
        &[
            ("prompt", &usage.prompt_tokens.to_string()),
            ("completion", &usage.completion_tokens.to_string()),
            ("total", &usage.total().to_string()),
        ],
    )
}
