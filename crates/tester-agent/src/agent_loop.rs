use crate::AgentContext;
use crate::compact::maybe_compact;
use crate::progress::Progress;
use crate::session::AgentSession;
use crate::tools::{ToolKind, ToolRegistry};
use anyhow::{Result, bail};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tester_llm::{ChatMessage, Usage};
use tracing::{info, warn};

pub type OnDelta = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone)]
pub struct AgentOptions {
    pub max_turns: u32,
    pub max_tool_calls: u32,
    pub deadline_secs: u64,
    pub stream: bool,
    pub on_delta: Option<OnDelta>,
}

impl std::fmt::Debug for AgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentOptions")
            .field("max_turns", &self.max_turns)
            .field("max_tool_calls", &self.max_tool_calls)
            .field("deadline_secs", &self.deadline_secs)
            .field("stream", &self.stream)
            .field("on_delta", &self.on_delta.as_ref().map(|_| "Some(..)"))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub final_text: String,
    pub usage: Usage,
    pub turns: u32,
    pub tool_calls: u32,
    pub cancelled: bool,
}

pub struct AgentLoop<'a> {
    ctx: &'a AgentContext,
    registry: ToolRegistry,
    opts: AgentOptions,
}

impl<'a> AgentLoop<'a> {
    pub fn new(ctx: &'a AgentContext, registry: ToolRegistry, opts: AgentOptions) -> Self {
        Self {
            ctx,
            registry,
            opts,
        }
    }

    pub fn run(&mut self, session: &mut AgentSession) -> Result<AgentOutcome> {
        let started = Instant::now();
        let deadline = Duration::from_secs(self.opts.deadline_secs);
        let mut progress = Progress::new();
        let mut final_text = String::new();
        let mut cancelled = false;

        for turn in 1..=self.opts.max_turns {
            if self.ctx.cancel.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
            if started.elapsed() > deadline {
                warn!("deadline агента исчерпан");
                bail!("агент превысил deadline {}s", self.opts.deadline_secs);
            }

            maybe_compact(
                self.ctx.llm.as_ref(),
                session,
                self.ctx.config.run.max_context_chars,
            )?;

            progress.step(&format!("ход {turn}: вызов модели"));
            session.turns = session.turns.saturating_add(1);

            let tools = self.registry.specs();
            let result = if let Some(cb) = &self.opts.on_delta {
                let cb = std::sync::Arc::clone(cb);
                self.ctx
                    .llm
                    .chat(&session.messages, &tools, Some(&|delta: &str| cb(delta)))?
            } else if self.opts.stream
                && !tester_core::tty::is_active()
                && !crossterm::terminal::is_raw_mode_enabled().unwrap_or(false)
            {
                let result = self.ctx.llm.chat(
                    &session.messages,
                    &tools,
                    Some(&|delta: &str| {
                        print!("{delta}");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }),
                )?;
                println!();
                result
            } else {
                self.ctx.llm.chat(&session.messages, &tools, None)?
            };
            if let Some(u) = &result.usage {
                session.usage.add_assign(u);
            }

            if result.tool_calls.is_empty() {
                final_text = result.content.clone().unwrap_or_default();
                session
                    .messages
                    .push(ChatMessage::assistant(result.content, vec![]));
                progress.step("готово");
                break;
            }

            session.messages.push(ChatMessage::assistant(
                result.content.clone(),
                result.tool_calls.clone(),
            ));

            let mut read_batch = Vec::new();
            let mut rest = Vec::new();
            for tc in &result.tool_calls {
                match self.registry.get(&tc.function.name) {
                    Some(tool) if tool.kind() == ToolKind::ReadOnly => read_batch.push(tc.clone()),
                    _ => rest.push(tc.clone()),
                }
            }

            for tc in read_batch.into_iter().chain(rest) {
                if self.ctx.cancel.load(Ordering::SeqCst) {
                    cancelled = true;
                    break;
                }
                if session.tool_calls >= self.opts.max_tool_calls {
                    bail!("превышен max_tool_calls ({})", self.opts.max_tool_calls);
                }
                session.tool_calls = session.tool_calls.saturating_add(1);

                let tool_result = self.execute_one(&tc.function.name, &tc.function.arguments);
                let content = match tool_result {
                    Ok(s) => s,
                    Err(err) => format!("ошибка инструмента: {err:#}"),
                };
                info!(tool = %tc.function.name, "tool_result");
                progress.step(&format!("tool {}", tc.function.name));
                session
                    .messages
                    .push(ChatMessage::tool(&tc.id, truncate_result(&content)));
            }

            if cancelled {
                break;
            }

            if turn == self.opts.max_turns {
                final_text = result.content.unwrap_or_else(|| "(лимит ходов)".into());
            }
        }

        let _ = self.ctx.audit.persist(&self.ctx.profile.root);

        Ok(AgentOutcome {
            final_text,
            usage: session.usage.clone(),
            turns: session.turns,
            tool_calls: session.tool_calls,
            cancelled,
        })
    }

    fn execute_one(&self, name: &str, arguments: &str) -> Result<String> {
        let tool = self
            .registry
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("неизвестный tool: {name}"))?;
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or_else(|_| serde_json::json!({ "raw": arguments }));
        let preview = arguments.chars().take(120).collect::<String>();
        if !self.ctx.permissions.allow(tool.kind(), name, &preview) {
            return Ok("отклонено пользователем".into());
        }
        tool.execute(self.ctx, &args)
    }
}

fn truncate_result(s: &str) -> String {
    const MAX: usize = 24_000;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let t: String = s.chars().take(MAX).collect();
        format!("{t}\n... truncated")
    }
}
