use super::{Tool, ToolKind, arg_bool, arg_str, truncate};
use crate::AgentContext;
use anyhow::Result;
use serde_json::{Value, json};
use tester_security::{draft_patch, explain, render_text, scan_with_config};

pub struct SecurityScan;
impl Tool for SecurityScan {
    fn name(&self) -> &str {
        "security_scan"
    }
    fn description(&self) -> &str {
        "Defensive security scan: secrets, RCE sinks, config risks (read-only)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "deep": {"type": "boolean", "description": "Enable intra-file taint pass"},
                "max_chars": {"type": "integer"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let mut cfg = ctx.config.security.clone();
        if arg_bool(args, "deep", false) {
            cfg.deep = true;
        }
        let report = scan_with_config(&ctx.profile, &cfg, Some(cfg.deep))?;
        let max = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(12_000) as usize;
        Ok(truncate(&render_text(&report), max))
    }
}

pub struct SecurityExplain;
impl Tool for SecurityExplain {
    fn name(&self) -> &str {
        "security_explain"
    }
    fn description(&self) -> &str {
        "Explain a security finding by id (defensive remediation advice only)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "finding": {"type": "string"},
                "deep": {"type": "boolean"}
            },
            "required": ["finding"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let id = arg_str(args, "finding")?;
        let mut cfg = ctx.config.security.clone();
        if arg_bool(args, "deep", false) {
            cfg.deep = true;
        }
        let report = scan_with_config(&ctx.profile, &cfg, Some(cfg.deep))?;
        explain(&report, &id)
    }
}

pub struct SecurityPatch;
impl Tool for SecurityPatch {
    fn name(&self) -> &str {
        "security_patch"
    }
    fn description(&self) -> &str {
        "Draft or apply a defensive harden patch for a finding (write; confirm required)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "finding": {"type": "string"},
                "apply": {"type": "boolean", "description": "Write file (default false = dry-run)"},
                "use_llm": {"type": "boolean"},
                "deep": {"type": "boolean"}
            },
            "required": ["finding"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let id = arg_str(args, "finding")?;
        let apply = arg_bool(args, "apply", false);
        if ctx.dry_run && apply {
            return Ok("dry_run=true: patch not applied".into());
        }
        let mut cfg = ctx.config.security.clone();
        if arg_bool(args, "deep", false) {
            cfg.deep = true;
        }
        let report = scan_with_config(&ctx.profile, &cfg, Some(cfg.deep))?;
        let f = report
            .find(&id)
            .ok_or_else(|| anyhow::anyhow!("finding not found: {id}"))?
            .clone();
        let use_llm = arg_bool(args, "use_llm", false);
        let draft = draft_patch(
            &ctx.profile,
            &f,
            if use_llm {
                Some(ctx.llm.as_ref())
            } else {
                None
            },
            apply && !ctx.dry_run,
        )?;
        Ok(format!(
            "note: {}\napplied: {}\n\n{}",
            draft.note, draft.applied, draft.diff
        ))
    }
}
