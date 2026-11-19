use super::{Tool, ToolKind, arg_str_opt, truncate};
use crate::agent::AgentContext;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::process::Command;

fn git(ctx: &AgentContext, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(&ctx.profile.root)
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if !out.status.success() {
        bail!("git {} failed: {stderr}", args.join(" "));
    }
    Ok(stdout)
}

pub struct GitStatus;
impl Tool for GitStatus {
    fn name(&self) -> &str {
        "git_status"
    }
    fn description(&self) -> &str {
        "git status --short"
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, _args: &Value) -> Result<String> {
        let out = git(ctx, &["status", "--short"])?;
        ctx.audit.record("git_status", ".", "ok");
        Ok(if out.trim().is_empty() {
            "(чисто)".into()
        } else {
            out
        })
    }
}

pub struct GitDiff;
impl Tool for GitDiff {
    fn name(&self) -> &str {
        "git_diff"
    }
    fn description(&self) -> &str {
        "git diff; опционально base ref (как --base)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "base": {"type": "string", "description": "ref для diff base...HEAD"},
                "max_chars": {"type": "integer"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let max = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(24_000) as usize;
        let out = if let Some(base) = arg_str_opt(args, "base") {
            git(ctx, &["diff", &format!("{base}...HEAD")])?
        } else {
            git(ctx, &["diff"])?
        };
        ctx.audit.record("git_diff", "ok", "");
        Ok(truncate(&out, max))
    }
}
