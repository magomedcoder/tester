use crate::cache;
use crate::types::WikiScenario;
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};
use tester_core::model::{Gap, PlanItem, PlannedCase, Symbol, TestKind};

pub fn plan_items_from_wiki(
    project_root: &Path,
    version: Option<&str>,
    max_items: usize,
) -> anyhow::Result<Vec<PlanItem>> {
    let scenarios = cache::load_scenarios(project_root)?;
    let mut items = Vec::new();
    for sc in scenarios.scenarios {
        if sc.kind == "non_ui" {
            continue;
        }
        if sc.coverage == "covered" {
            continue;
        }
        if let Some(v) = version
            && sc.version != v
            && sc.version != "default"
        {
            continue;
        }
        items.push(scenario_to_plan_item(&sc));
        if items.len() >= max_items.max(1) {
            break;
        }
    }
    Ok(items)
}

pub(crate) fn scenario_to_plan_item(sc: &WikiScenario) -> PlanItem {
    let cases: Vec<PlannedCase> = sc
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut desc = s.action.clone();
            if let Some(t) = &s.target {
                desc.push_str(": ");
                desc.push_str(t);
            }
            PlannedCase {
                name: format!("step_{}", i + 1),
                description: desc.clone(),
                input: s.value.clone().unwrap_or_default(),
                expectation: if s.action == "expect" {
                    s.target.clone().unwrap_or_default()
                } else {
                    String::new()
                },
            }
        })
        .collect();
    let mut context = String::new();
    if !sc.preconditions.is_empty() {
        context.push_str(i18n::t(Msg::WikiPreconditionsHeader));
        context.push('\n');
        for p in &sc.preconditions {
            context.push_str(&format!("- {p}\n"));
        }
    }
    context.push_str(&i18n::tr(Msg::WikiScenarioLabel, &[("title", &sc.title)]));
    context.push('\n');
    context.push_str(&i18n::tr(
        Msg::WikiCoverageLabel,
        &[("coverage", &sc.coverage)],
    ));
    context.push('\n');
    if !sc.source_pages.is_empty() {
        context.push_str(&i18n::tr(
            Msg::WikiSourcesLabel,
            &[("sources", &sc.source_pages.join(", "))],
        ));
        context.push('\n');
    }

    PlanItem {
        kind: TestKind::Ui,
        gap: Gap {
            symbol: Symbol {
                name: sc.id.clone(),
                kind: "ui".into(),
                file: PathBuf::from("e2e"),
                start_line: 1,
                end_line: 1,
                signature: sc.title.clone(),
                body: String::new(),
                is_public: true,
                uses_io: true,
                branch_score: 2,
                qualname: sc.id.clone(),
                visibility: String::new(),
                parent: None,
                docstring: sc.title.clone(),
                start_byte: 0,
            },
            kind: TestKind::Ui,
            reason: i18n::tr(Msg::WikiScenarioReason, &[("title", &sc.title)]),
            priority: 70,
        },
        target_path: PathBuf::from(format!("e2e/wiki_{}.spec.ts", sanitize_file(&sc.id))),
        cases,
        append_to_existing: false,
        bug_context: Some(context),
    }
}

fn sanitize_file(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect()
}
