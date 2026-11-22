use super::{Tool, ToolKind, arg_bool, arg_str, arg_str_opt, truncate};
use crate::AgentContext;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use tester_core::model::{ItemStatus, TestKind};
use tester_lang::discover::discover;
use tester_pipeline::apply::apply_draft;
use tester_pipeline::generate::{generate_draft, generate_repair};
use tester_pipeline::plan::{PlanOptions, build_plan_with};
use tester_pipeline::plan_store::{load_plan, save_plan};
use tester_pipeline::repair::{
    LastRepairState, append_history, load_state, save_state, state_path,
};
use tester_pipeline::validate::validate_draft;

pub struct ScanProject;
impl Tool for ScanProject {
    fn name(&self) -> &str {
        "scan_project"
    }
    fn description(&self) -> &str {
        "Профиль проекта: язык, команды тестов, capabilities"
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, _args: &Value) -> Result<String> {
        let profile = discover(&ctx.profile.root)?;
        ctx.audit
            .record("scan_project", profile.language.as_str(), "ok");
        Ok(serde_json::to_string_pretty(&profile)?)
    }
}

pub struct BuildPlan;
impl Tool for BuildPlan {
    fn name(&self) -> &str {
        "build_plan"
    }
    fn description(&self) -> &str {
        "Построить план покрытия; сохраняет .tester/plan.json"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {"type": "string", "description": "all|unit|integration|ui|contract"},
                "base": {"type": "string"},
                "bug": {"type": "string"},
                "append": {"type": "boolean"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let kind: TestKind = arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow!(e))?;
        let bug = arg_str_opt(args, "bug");
        let frames = bug
            .as_ref()
            .map(|t| tester_core::stacktrace::parse_stacktrace(t))
            .unwrap_or_default();
        let opts = PlanOptions {
            diff_base: arg_str_opt(args, "base"),
            bug_text: bug,
            prefer_append: arg_bool(args, "append", false),
            max_items: None,
            coverage_path: None,
            stack_frames: frames,
            reproduce_only: false,
        };
        let plan = build_plan_with(&ctx.profile, &ctx.config, kind, &opts)?;
        save_plan(&ctx.profile.root, &plan)?;
        ctx.audit.record(
            "build_plan",
            kind.as_str(),
            &format!("{} items", plan.items.len()),
        );
        Ok(serde_json::to_string_pretty(&plan)?)
    }
}

pub struct WriteTest;
impl Tool for WriteTest {
    fn name(&self) -> &str {
        "write_test"
    }
    fn description(&self) -> &str {
        "Сгенерировать тест для символа из текущего/сохранённого плана и записать (или dry-run)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "symbol": {"type": "string"},
                "kind": {"type": "string"},
                "update": {"type": "boolean"}
            },
            "required": ["symbol"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let symbol = arg_str(args, "symbol")?;
        let kind: TestKind = arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow!(e))?;
        let update = arg_bool(args, "update", true);
        let plan = load_plan(&ctx.profile.root).or_else(|_| {
            build_plan_with(&ctx.profile, &ctx.config, kind, &PlanOptions::default())
        })?;
        let item = plan
            .items
            .iter()
            .find(|i| i.gap.symbol.name == symbol && (kind == TestKind::All || i.kind == kind))
            .cloned()
            .ok_or_else(|| anyhow!("символ '{symbol}' не найден в плане"))?;
        let draft = generate_draft(ctx.llm.as_ref(), &ctx.profile, &ctx.config, &item)?;
        let write = !ctx.dry_run;
        let out = apply_draft(&ctx.profile, &draft, write, update)?;
        ctx.audit.record(
            "write_test",
            &symbol,
            if write { "written" } else { "dry_run" },
        );
        Ok(format!(
            "path: {}\noutcome: {out:?}\npreview:\n{}",
            draft.path.display(),
            truncate(&draft.content, 4_000)
        ))
    }
}

pub struct RunTests;
impl Tool for RunTests {
    fn name(&self) -> &str {
        "run_tests"
    }
    fn description(&self) -> &str {
        "Прогнать тест по пути через validate_draft"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "kind": {"type": "string"},
                "symbol": {"type": "string"},
                "filter_name": {"type": "string"}
            },
            "required": ["path"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        use tester_core::model::Draft;
        let path = arg_str(args, "path")?;
        let kind: TestKind = arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow!(e))?;
        if ctx.dry_run {
            return Ok(format!("dry-run validate {path}"));
        }
        let content = std::fs::read_to_string(ctx.profile.root.join(&path)).unwrap_or_default();
        let draft = Draft {
            path: path.clone().into(),
            content,
            kind,
            symbol_name: arg_str_opt(args, "symbol").unwrap_or_default(),
            filter_name: arg_str_opt(args, "filter_name").unwrap_or_default(),
            append: false,
        };
        let result = validate_draft(&ctx.profile, &ctx.config, &draft)?;
        ctx.audit
            .record("run_tests", &path, if result.ok { "pass" } else { "fail" });
        Ok(format!(
            "ok: {}\ncommand: {}\nstdout:\n{}\nstderr:\n{}",
            result.ok,
            result.command,
            truncate(&result.stdout, 6_000),
            truncate(&result.stderr, 4_000)
        ))
    }
}

pub struct RepairTest;
impl Tool for RepairTest {
    fn name(&self) -> &str {
        "repair_test"
    }
    fn description(&self) -> &str {
        "Один шаг repair: по ошибке прогона починить тест через LLM"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "symbol": {"type": "string"},
                "error": {"type": "string", "description": "Текст ошибки; если нет - из last_repair"}
            },
            "required": ["symbol"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let symbol = arg_str(args, "symbol")?;
        let plan = load_plan(&ctx.profile.root).or_else(|_| {
            build_plan_with(
                &ctx.profile,
                &ctx.config,
                TestKind::All,
                &PlanOptions::default(),
            )
        })?;
        let item = plan
            .items
            .iter()
            .find(|i| i.gap.symbol.name == symbol)
            .cloned()
            .or_else(|| {
                load_state(&ctx.profile.root).ok().and_then(|st| {
                    plan.items
                        .iter()
                        .find(|i| i.gap.symbol.name == st.item_symbol)
                        .cloned()
                })
            })
            .ok_or_else(|| anyhow!("нет plan item для {symbol}"))?;

        let (mut draft, error) = if let Ok(st) = load_state(&ctx.profile.root) {
            (st.draft, arg_str_opt(args, "error").unwrap_or(st.error))
        } else {
            bail!("нет .tester/last_repair.json - сначала run_tests с падением");
        };

        if ctx.dry_run {
            return Ok(format!(
                "dry-run repair {symbol}\nerror preview:\n{}",
                truncate(&error, 1000)
            ));
        }

        draft = generate_repair(ctx.llm.as_ref(), &ctx.profile, &item, &draft, &error)?;
        apply_draft(&ctx.profile, &draft, true, true)?;
        let result = validate_draft(&ctx.profile, &ctx.config, &draft)?;
        let err_blob = format!("{}\n{}", result.stdout, result.stderr);
        save_state(
            &ctx.profile.root,
            &LastRepairState {
                root: ctx.profile.root.clone(),
                draft: draft.clone(),
                error: err_blob.clone(),
                item_symbol: item.gap.symbol.name.clone(),
                kind: item.kind,
            },
        )?;
        append_history(
            &ctx.profile.root,
            &symbol,
            if result.ok {
                ItemStatus::Passed
            } else {
                ItemStatus::Unfixed
            },
            &err_blob,
        )?;
        if result.ok {
            let _ = std::fs::remove_file(state_path(&ctx.profile.root));
        }
        ctx.audit.record(
            "repair_test",
            &symbol,
            if result.ok { "passed" } else { "still_failing" },
        );
        Ok(format!(
            "ok: {}\npath: {}\ncommand: {}\n{}",
            result.ok,
            draft.path.display(),
            result.command,
            truncate(&err_blob, 4_000)
        ))
    }
}

pub struct DebugFailure;
impl Tool for DebugFailure {
    fn name(&self) -> &str {
        "debug_failure"
    }
    fn description(&self) -> &str {
        "Разобрать traceback/лог прогона: класс ошибки, фреймы, совет (debug --reproduce-only)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "log": {"type": "string", "description": "Текст ошибки / traceback"}
            },
            "required": ["log"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let log = arg_str(args, "log")?;
        let out = tester_pipeline::debug::debug_failure_tool_in(&ctx.profile.root, &log);
        ctx.audit.record("debug_failure", "log", "ok");
        Ok(out)
    }
}
