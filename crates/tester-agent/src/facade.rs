use crate::AgentContext;
use anyhow::Result;
use std::path::Path;
use tester_core::model::{TestKind, TestPlan};
use tester_pipeline::plan::{PlanOptions, build_plan_with};
use tester_pipeline::plan_store::save_plan;

pub fn build_plan(ctx: &AgentContext, kind: TestKind) -> Result<TestPlan> {
    build_plan_with(&ctx.profile, &ctx.config, kind, &PlanOptions::default())
}

pub fn build_and_save_plan(ctx: &AgentContext, kind: TestKind) -> Result<TestPlan> {
    let plan = build_plan(ctx, kind)?;
    save_plan(&ctx.profile.root, &plan)?;
    Ok(plan)
}

pub fn resolve_symbol_attach(root: &Path, name: &str) -> Option<(String, String)> {
    let profile = tester_lang::discover::discover(root).ok()?;
    let symbols = tester_lang::index::index_symbols(&profile).ok()?;
    let q = tester_analysis::search::SymbolQuery {
        name: name.into(),
        max: 1,
        ..Default::default()
    };
    let hit = tester_analysis::search::find_symbol(&symbols, &q)
        .into_iter()
        .next()?;
    let path = hit.path.to_string_lossy().to_string();
    let preview = tester_analysis::context::read_range(
        root,
        &path,
        hit.start_line,
        hit.end_line.min(hit.start_line + 40),
    )
    .unwrap_or_default();
    let label = format!(
        "{}#L{}-{}",
        path,
        hit.start_line,
        hit.end_line.min(hit.start_line + 40)
    );
    Some((label, preview))
}
