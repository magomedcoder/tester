use crate::agent::session::{AgentSession, clear_session, load_session, save_session};
use crate::agent::{AgentContext, run_one_shot};
use crate::llm::ChatMessage;
use crate::model::TestKind;
use crate::plan::{PlanOptions, build_plan_with};
use crate::plan_store::save_plan;
use anyhow::Result;
use std::io::{self, Write};

pub fn run_repl(ctx: &AgentContext) -> Result<()> {
    eprintln!("tester chat - REPL агента. /help для команд, Ctrl+C выход.");
    let mut session = load_session(&ctx.profile.root).unwrap_or_else(|_| AgentSession::new());
    if session.messages.is_empty() {
        session
            .messages
            .push(ChatMessage::system(crate::agent::prompt::system_prompt(
                &ctx.profile,
                &ctx.config,
            )));
    }

    loop {
        if ctx.cancel.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        eprint!("tester> ");
        let _ = io::stderr().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(cmd) = line.strip_prefix('/') {
            match handle_slash(ctx, &mut session, cmd)? {
                SlashAction::Continue => continue,
                SlashAction::Quit => break,
            }
        }

        let outcome = {
            save_session(&ctx.profile.root, &session)?;
            run_one_shot(ctx, line, true)?
        };
        session = load_session(&ctx.profile.root).unwrap_or(session);
        println!("{}", outcome.final_text);
        eprintln!(
            "- turns={} tools={} tokens≈{} {}",
            outcome.turns,
            outcome.tool_calls,
            outcome.usage.total(),
            if outcome.cancelled { "(cancelled)" } else { "" }
        );
        eprintln!("{}", ctx.audit.summary());
    }

    save_session(&ctx.profile.root, &session)?;
    let _ = ctx.audit.persist(&ctx.profile.root);
    Ok(())
}

enum SlashAction {
    Continue,
    Quit,
}

fn handle_slash(ctx: &AgentContext, session: &mut AgentSession, cmd: &str) -> Result<SlashAction> {
    let mut parts = cmd.split_whitespace();
    let name = parts.next().unwrap_or("");
    match name {
        "help" | "h" | "?" => {
            eprintln!(
                "Команды:\n  /scan\n  /plan [kind]\n  /run <prompt>\n  /config\n  /clear\n  /audit\n  /exit"
            );
            Ok(SlashAction::Continue)
        }
        "exit" | "quit" | "q" => Ok(SlashAction::Quit),
        "clear" => {
            session.messages.clear();
            session
                .messages
                .push(ChatMessage::system(crate::agent::prompt::system_prompt(
                    &ctx.profile,
                    &ctx.config,
                )));
            clear_session(&ctx.profile.root)?;
            eprintln!("сессия очищена");
            Ok(SlashAction::Continue)
        }
        "config" => {
            eprintln!(
                "model={} base_url={} dry_run={} agent_auto_approve={} allow_docker={} android_opt_in={}",
                ctx.config.llm.model,
                ctx.config.llm.base_url,
                ctx.dry_run,
                ctx.config.run.agent_auto_approve,
                ctx.config.run.allow_docker,
                ctx.config.run.android_opt_in
            );
            Ok(SlashAction::Continue)
        }
        "audit" => {
            eprintln!("{}", ctx.audit.summary());
            Ok(SlashAction::Continue)
        }
        "scan" => {
            let profile = crate::discover::discover(&ctx.profile.root)?;
            println!(
                "язык={} пакет={} cmd={}",
                profile.language.as_str(),
                profile.package_name,
                profile.test_command
            );
            Ok(SlashAction::Continue)
        }
        "plan" => {
            let kind: TestKind = parts
                .next()
                .unwrap_or("unit")
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;
            let plan = build_plan_with(&ctx.profile, &ctx.config, kind, &PlanOptions::default())?;
            save_plan(&ctx.profile.root, &plan)?;
            for item in &plan.items {
                println!(
                    "- [{}] {} -> {}",
                    item.kind.as_str(),
                    item.gap.symbol.name,
                    item.target_path.display()
                );
            }
            if plan.items.is_empty() {
                println!("нет целей");
            }
            Ok(SlashAction::Continue)
        }
        "run" => {
            let prompt = cmd.strip_prefix("run").unwrap_or("").trim();
            if prompt.is_empty() {
                eprintln!("usage: /run <prompt>");
                return Ok(SlashAction::Continue);
            }
            save_session(&ctx.profile.root, session)?;
            let outcome = run_one_shot(ctx, prompt, true)?;
            *session = load_session(&ctx.profile.root).unwrap_or_else(|_| session.clone());
            println!("{}", outcome.final_text);
            Ok(SlashAction::Continue)
        }
        other => {
            eprintln!("неизвестная команда /{other}; /help");
            Ok(SlashAction::Continue)
        }
    }
}
