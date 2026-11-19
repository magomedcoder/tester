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

pub use agent_loop::{AgentLoop, AgentOptions, AgentOutcome};
pub use audit::AuditLog;
pub use permissions::{PermissionMode, PermissionPolicy};
pub use progress::Progress;
pub use repl::run_repl;
pub use session::{AgentSession, load_session, save_session};

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

pub fn run_one_shot(ctx: &AgentContext, prompt: &str, resume: bool) -> Result<AgentOutcome> {
    let mut session = if resume {
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
        stream: ctx.stream,
    };
    let registry = tools::default_registry(ctx);
    let mut engine = AgentLoop::new(ctx, registry, opts);
    let outcome = engine.run(&mut session)?;
    save_session(&ctx.profile.root, &session)?;
    Ok(outcome)
}
