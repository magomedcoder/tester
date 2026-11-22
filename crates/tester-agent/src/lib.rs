#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if,
    )]

mod agent_loop;
mod audit;
mod compact;
pub mod facade;
mod permissions;
mod progress;
mod prompt;
mod repl;
mod session;
pub mod session_store;
pub mod tools;

#[cfg(test)]
mod tests;

pub use agent_loop::{AgentOptions, AgentOutcome, OnDelta};
pub(crate) use agent_loop::AgentLoop;
pub use audit::AuditLog;
pub use permissions::PermissionPolicy;
pub use repl::run_repl;
pub use session::AgentSession;
pub(crate) use session::{load_session, save_session};

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tester_core::config::Config;
use tester_core::model::ProjectProfile;
use tester_llm::Llm;

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

pub fn open_context_with_policy(
    path: &std::path::Path,
    dry_run: bool,
    stream: bool,
    tui_safe: bool,
) -> Result<AgentContext> {
    let mut profile = tester_lang::discover::discover(path)?;
    tester_lang::adapter::ensure_scaffold(&profile)?;
    if profile.language == tester_core::model::Language::Cpp {
        profile = tester_lang::discover::discover(path)?;
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
    let llm = tester_llm::create_client(&config.llm)?;
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
            .push(tester_llm::ChatMessage::system(prompt::system_prompt(
                &ctx.profile,
                &ctx.config,
            )));
    }
    session.messages.push(tester_llm::ChatMessage::user(prompt));

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
