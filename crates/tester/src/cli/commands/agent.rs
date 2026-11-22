use crate::cli::helpers::{build_agent_ctx, mirror_cancel};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tester_agent::{run_one_shot, run_repl};
use tester_core::error::{AppError, ExitCode};
use tester_core::i18n::{self, Msg};
use tester_pipeline::report::render_usage;

pub(crate) fn cmd_agent(
    path: &PathBuf,
    prompt: &str,
    dry_run: bool,
    resume: bool,
    stream: bool,
) -> Result<ExitCode> {
    let ctx = build_agent_ctx(path, dry_run, stream)?;
    mirror_cancel(Arc::clone(&ctx.cancel));

    let outcome = run_one_shot(&ctx, prompt, resume)?;
    println!("{}", outcome.final_text);
    eprintln!("- {}", render_usage(&outcome.usage));
    let status = if outcome.cancelled {
        i18n::t(Msg::SessionOutcomeCancelled)
    } else {
        i18n::t(Msg::SessionOutcomeOk)
    };
    eprintln!(
        "- {}",
        i18n::tr(
            Msg::SessionOutcomeLine,
            &[
                ("turns", &outcome.turns.to_string()),
                ("tools", &outcome.tool_calls.to_string()),
                ("status", &format!(" {status}")),
            ],
        )
    );
    eprintln!("{}", ctx.audit.summary());
    Ok(if outcome.cancelled {
        ExitCode::ERROR
    } else {
        ExitCode::OK
    })
}

pub(crate) fn cmd_chat(path: &PathBuf, dry_run: bool, stream: bool) -> Result<ExitCode> {
    let ctx = build_agent_ctx(path, dry_run, stream)?;
    mirror_cancel(Arc::clone(&ctx.cancel));
    run_repl(&ctx)?;
    Ok(ExitCode::OK)
}

pub(crate) fn cmd_tui(
    path: Option<PathBuf>,
    dry_run: bool,
    session: Option<String>,
    plain: bool,
) -> Result<ExitCode> {
    if plain {
        let path = path.ok_or(AppError::PlainNeedsPath)?;
        return cmd_chat(&path, dry_run, true);
    }
    tester_tui::run(tester_tui::TuiOptions {
        path,
        dry_run,
        session_id: session,
    })?;
    Ok(ExitCode(0))
}
