#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
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

pub(crate) use agent_loop::AgentLoop;
pub use agent_loop::{AgentOptions, AgentOutcome, OnDelta};
pub use audit::AuditLog;
pub use permissions::PermissionPolicy;
pub use repl::run_repl;
pub use session::AgentSession;
pub(crate) use session::{load_session, save_session};

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
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
    open_context_ex(path, dry_run, stream, tui_safe, None)
}

pub fn open_context_ex(
    path: &std::path::Path,
    dry_run: bool,
    stream: bool,
    tui_safe: bool,
    model_override: Option<&str>,
) -> Result<AgentContext> {
    let mut profile = tester_lang::discover::discover(path)?;
    tester_lang::adapter::ensure_scaffold(&profile)?;
    if profile.language == tester_core::model::Language::Cpp {
        profile = tester_lang::discover::discover(path)?;
    }
    let mut config = Config::load(&profile.root)?;
    if config.run.android_opt_in {
        profile.android_opt_in = true;
    }
    if config.run.allow_docker {
        profile.notes.push(i18n::t(Msg::NoteDockerAllowed).into());
    }
    let had_model = !config.llm.model.trim().is_empty();
    let explicit = model_override
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string);
    if let Some(m) = &explicit {
        config.llm.model = m.clone();
    }
    let llm = match tester_llm::ensure_llm_config(config.llm.clone()) {
        Ok(cfg) => {
            let should_persist = explicit.is_some() || (!had_model && !cfg.model.trim().is_empty());
            config.llm = cfg;
            if should_persist {
                if let Err(err) = tester_core::config::persist_llm_model(&config.llm.model) {
                    if !tui_safe {
                        return Err(err);
                    }
                }
            }
            tester_llm::create_client_with_model(&config.llm)?
        }
        Err(err) => {
            let soft = tui_safe
                && err
                    .downcast_ref::<tester_core::error::AppError>()
                    .is_some_and(|e| {
                        matches!(
                            e,
                            tester_core::error::AppError::ModelNeedsSelection { .. }
                                | tester_core::error::AppError::NoModels
                        )
                    });
            if soft {
                tester_llm::unset_model_client()
            } else {
                return Err(err);
            }
        }
    };
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
