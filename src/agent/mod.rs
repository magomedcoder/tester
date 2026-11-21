mod agent_loop;
mod audit;
mod compact;
mod permissions;
mod progress;
mod prompt;
mod repl;
mod session;
pub mod tools;

#[cfg(test)]
mod tests;

pub use agent_loop::{AgentLoop, AgentOptions, AgentOutcome, OnDelta};
pub use audit::AuditLog;
pub use permissions::{ConfirmFn, PermissionMode, PermissionPolicy};
pub use progress::Progress;
pub use repl::run_repl;
pub use session::{AgentSession, clear_session, load_session, save_session};

use crate::config::Config;
use crate::llm::Llm;
use crate::model::ProjectProfile;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct AgentContext {
    pub profile: ProjectProfile,
    pub config: Config,
    pub llm: Arc<dyn Llm>,
    pub dry_run: bool,
    pub yes: bool,
    pub stream: bool,
    pub cancel: Arc<AtomicBool>,
    pub audit: AuditLog,
    pub permissions: PermissionPolicy,
}

pub fn open_context(path: &std::path::Path, dry_run: bool, stream: bool) -> Result<AgentContext> {
    open_context_with_policy(path, dry_run, stream, false)
}

pub fn open_context_with_policy(
    path: &std::path::Path,
    dry_run: bool,
    stream: bool,
    tui_safe: bool,
) -> Result<AgentContext> {
    let mut profile = crate::discover::discover(path)?;
    crate::adapter::ensure_scaffold(&profile)?;
    if profile.language == crate::model::Language::Cpp {
        profile = crate::discover::discover(path)?;
    }
    let config = Config::load(&profile.root)?;
    if config.run.android_opt_in {
        profile.android_opt_in = true;
    }
    if config.run.allow_docker {
        profile.notes.push(
            "allow_docker=true: tool docker_info доступен агенту; внешние сервисы по-прежнему не поднимаются автоматически"
                .into(),
        );
    }
    let llm = crate::llm::create_client(&config.llm)?;
    let yes = config.run.agent_auto_approve;
    let permissions = if tui_safe {
        PermissionPolicy::for_tui(yes, dry_run)
    } else {
        PermissionPolicy::from_flags(yes, dry_run)
    };
    Ok(AgentContext {
        profile,
        config,
        llm,
        dry_run,
        yes,
        stream,
        cancel: Arc::new(AtomicBool::new(false)),
        audit: AuditLog::new(),
        permissions,
    })
}

pub fn run_one_shot(ctx: &AgentContext, prompt: &str, resume: bool) -> Result<AgentOutcome> {
    run_one_shot_ex(
        ctx,
        prompt,
        RunOneShotOpts {
            resume,
            on_delta: None,
        },
    )
}

pub struct RunOneShotOpts {
    pub resume: bool,
    pub on_delta: Option<OnDelta>,
}

pub fn run_one_shot_ex(
    ctx: &AgentContext,
    prompt: &str,
    run_opts: RunOneShotOpts,
) -> Result<AgentOutcome> {
    let mut session = if run_opts.resume {
        load_session(&ctx.profile.root).unwrap_or_else(|_| AgentSession::new())
    } else {
        AgentSession::new()
    };
    if session.messages.is_empty() {
        session
            .messages
            .push(crate::llm::ChatMessage::system(prompt::system_prompt(
                &ctx.profile,
                &ctx.config,
            )));
    }
    session.messages.push(crate::llm::ChatMessage::user(prompt));

    let opts = AgentOptions {
        max_turns: ctx.config.run.max_agent_turns,
        max_tool_calls: ctx.config.run.max_tool_calls,
        deadline_secs: ctx.config.run.agent_deadline_secs,
        stream: ctx.stream || run_opts.on_delta.is_some(),
        on_delta: run_opts.on_delta,
    };
    let registry = tools::default_registry(ctx);
    let mut engine = AgentLoop::new(ctx, registry, opts);
    let outcome = engine.run(&mut session)?;
    save_session(&ctx.profile.root, &session)?;
    Ok(outcome)
}
