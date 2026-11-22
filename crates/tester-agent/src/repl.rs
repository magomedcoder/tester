use crate::session::{AgentSession, clear_session, load_session, save_session};
use crate::tools;
use crate::{AgentContext, run_one_shot};
use anyhow::Result;
use std::io::{self, Write};
use tester_core::i18n::{self, Msg};
use tester_core::model::TestKind;
use tester_providers::ChatMessage;
use tester_pipeline::plan::{PlanOptions, build_plan_with};
use tester_pipeline::plan_store::save_plan;

pub fn run_repl(ctx: &AgentContext) -> Result<()> {
    eprintln!("{}", i18n::t(Msg::ReplBanner));
    let mut session = load_session(&ctx.profile.root).unwrap_or_else(|_| AgentSession::new());
    if session.messages.is_empty() {
        session
            .messages
            .push(ChatMessage::system(crate::prompt::system_prompt(
                &ctx.profile,
                &ctx.config,
            )));
    }

    loop {
        if ctx.cancel.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        eprint!("{}", i18n::t(Msg::ReplPrompt));
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
            "{}",
            i18n::tr(
                Msg::ReplTurnsLine,
                &[
                    ("turns", &outcome.turns.to_string()),
                    ("tools", &outcome.tool_calls.to_string()),
                    ("tokens", &outcome.usage.total().to_string()),
                    (
                        "status",
                        if outcome.cancelled {
                            i18n::t(Msg::SessionOutcomeCancelled)
                        } else {
                            ""
                        }
                    ),
                ],
            )
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
            eprintln!("{}", i18n::t(Msg::ReplSlashHelp));
            Ok(SlashAction::Continue)
        }
        "exit" | "quit" | "q" => Ok(SlashAction::Quit),
        "clear" => {
            session.messages.clear();
            session
                .messages
                .push(ChatMessage::system(crate::prompt::system_prompt(
                    &ctx.profile,
                    &ctx.config,
                )));
            clear_session(&ctx.profile.root)?;
            eprintln!("{}", i18n::t(Msg::SessionCleared));
            Ok(SlashAction::Continue)
        }
        "audit" => {
            eprintln!("{}", ctx.audit.summary());
            Ok(SlashAction::Continue)
        }
        "scan" => {
            let profile = tester_lang::discover::discover(&ctx.profile.root)?;
            println!(
                "{}",
                i18n::tr(
                    Msg::ScanSummary,
                    &[
                        ("lang", profile.language.as_str()),
                        ("pkg", &profile.package_name),
                        ("cmd", &profile.test_command),
                    ],
                )
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
                println!("{}", i18n::t(Msg::NoGoals));
            }
            Ok(SlashAction::Continue)
        }
        "run" => {
            let prompt = cmd.strip_prefix("run").unwrap_or("").trim();
            if prompt.is_empty() {
                eprintln!("{}", i18n::t(Msg::SessionUsageRun));
                return Ok(SlashAction::Continue);
            }
            save_session(&ctx.profile.root, session)?;
            let outcome = run_one_shot(ctx, prompt, true)?;
            *session = load_session(&ctx.profile.root).unwrap_or_else(|_| session.clone());
            println!("{}", outcome.final_text);
            Ok(SlashAction::Continue)
        }
        "grep" => {
            let pattern = cmd.strip_prefix("grep").unwrap_or("").trim();
            if pattern.is_empty() {
                eprintln!("{}", i18n::t(Msg::UsageGrep));
                return Ok(SlashAction::Continue);
            }
            let args = serde_json::json!({"pattern": pattern});
            let out = tools::default_registry(ctx)
                .get("grep")
                .ok_or_else(|| anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "grep")])))?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        "symbol" => {
            let name = cmd.strip_prefix("symbol").unwrap_or("").trim();
            if name.is_empty() {
                eprintln!("{}", i18n::t(Msg::UsageSymbol));
                return Ok(SlashAction::Continue);
            }
            let args = serde_json::json!({"name": name});
            let out = tools::default_registry(ctx)
                .get("find_symbol")
                .ok_or_else(|| {
                    anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "find_symbol")]))
                })?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        "refs" => {
            let name = cmd.strip_prefix("refs").unwrap_or("").trim();
            if name.is_empty() {
                eprintln!("{}", i18n::t(Msg::UsageRefs));
                return Ok(SlashAction::Continue);
            }
            let args = serde_json::json!({"name": name});
            let out = tools::default_registry(ctx)
                .get("find_references")
                .ok_or_else(|| {
                    anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "find_references")]))
                })?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        "search" => {
            let q = cmd.strip_prefix("search").unwrap_or("").trim();
            if q.is_empty() {
                eprintln!("{}", i18n::t(Msg::UsageSearch));
                return Ok(SlashAction::Continue);
            }
            let args = serde_json::json!({"query": q});
            let out = tools::default_registry(ctx)
                .get("semantic_search")
                .ok_or_else(|| {
                    anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "semantic_search")]))
                })?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        "index" => {
            let sub = parts.next().unwrap_or("status");
            if sub == "rebuild" {
                let syms = tester_lang::index::rebuild_index(&ctx.profile)?;
                println!(
                    "{}",
                    i18n::tr(Msg::IndexRebuilt, &[("n", &syms.len().to_string())])
                );
            } else {
                let args = serde_json::json!({});
                let out = tools::default_registry(ctx)
                    .get("index_status")
                    .ok_or_else(|| {
                        anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "index_status")]))
                    })?
                    .execute(ctx, &args)?;
                println!("{out}");
            }
            Ok(SlashAction::Continue)
        }
        "map" | "list_files" => {
            let args = serde_json::json!({});
            let out = tools::default_registry(ctx)
                .get("project_map")
                .ok_or_else(|| {
                    anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "project_map")]))
                })?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        "show_file" => {
            let path = parts.next().unwrap_or("");
            if path.is_empty() {
                eprintln!("{}", i18n::t(Msg::UsageShowFile));
                return Ok(SlashAction::Continue);
            }
            let args = serde_json::json!({"path": path});
            let out = tools::default_registry(ctx)
                .get("outline_file")
                .ok_or_else(|| {
                    anyhow::anyhow!(i18n::tr(Msg::ToolMissing, &[("name", "outline_file")]))
                })?
                .execute(ctx, &args)?;
            println!("{out}");
            Ok(SlashAction::Continue)
        }
        other => {
            eprintln!("/{other}: {}", i18n::t(Msg::SessionUnknownCmd));
            Ok(SlashAction::Continue)
        }
    }
}
