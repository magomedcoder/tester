use super::{Tool, ToolKind, truncate};
use crate::adapter;
use crate::agent::AgentContext;
use crate::model::Draft;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::process::Command;
use std::time::Duration;

pub struct RunArgv;
impl Tool for RunArgv {
    fn name(&self) -> &str {
        "run_argv"
    }
    fn description(&self) -> &str {
        "Запустить whitelist argv адаптера для проверки теста (filter_name / path)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Относительный путь теста"},
                "kind": {"type": "string"},
                "filter_name": {"type": "string"},
                "symbol_name": {"type": "string"}
            },
            "required": ["path"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        use crate::model::TestKind;
        let path = super::arg_str(args, "path")?;
        let kind: TestKind = super::arg_str_opt(args, "kind")
            .unwrap_or_else(|| "unit".into())
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let draft = Draft {
            path: path.clone().into(),
            content: String::new(),
            kind,
            symbol_name: super::arg_str_opt(args, "symbol_name").unwrap_or_default(),
            filter_name: super::arg_str_opt(args, "filter_name").unwrap_or_default(),
            append: false,
        };
        let adapter = adapter::adapter_for(ctx.profile.language)?;
        let argv = adapter.validate_command(&ctx.profile, &draft);
        if argv.is_empty() {
            bail!("пустой argv адаптера");
        }

        if ctx.dry_run {
            ctx.audit.record("run_argv", &argv.join(" "), "dry_run");
            return Ok(format!("dry-run: {}", argv.join(" ")));
        }
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..])
            .current_dir(&ctx.profile.root)
            .env("CI", "1");
        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let summary = format!(
            "cmd: {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            argv.join(" "),
            output.status,
            truncate(&stdout, 8_000),
            truncate(&stderr, 4_000)
        );
        ctx.audit
            .record("run_argv", &argv.join(" "), &format!("{}", output.status));
        Ok(summary)
    }
}

pub struct DockerInfo;
impl Tool for DockerInfo {
    fn name(&self) -> &str {
        "docker_info"
    }
    fn description(&self) -> &str {
        "Проверить доступность Docker (только если run.allow_docker=true)"
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, _args: &Value) -> Result<String> {
        if !ctx.config.run.allow_docker {
            bail!("docker отключён: run.allow_docker=false");
        }
        let mut cmd = Command::new("docker");
        cmd.args(["info", "--format", "{{.ServerVersion}}"])
            .current_dir(&ctx.profile.root);
        let _ = Duration::from_secs(15);
        let output = cmd.output()?;
        if !output.status.success() {
            bail!(
                "docker недоступен: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
        ctx.audit.record("docker_info", &ver, "ok");
        Ok(format!("docker ok, version={ver}"))
    }
}
