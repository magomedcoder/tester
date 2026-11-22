use crate::cli::args::SessionsCmd;
use anyhow::Result;
use tester_core::error::{AppError, ExitCode};
use tester_core::i18n::{self, Msg};

pub(crate) fn cmd_sessions(action: SessionsCmd) -> Result<ExitCode> {
    use tester_agent::session_store::{self, PruneSpec};
    match action {
        SessionsCmd::List { workspace } => {
            let list = if let Some(ws) = workspace {
                session_store::list_for_workspace(&ws)?
            } else {
                session_store::list_all()?
            };
            for meta in list {
                println!(
                    "{}  {}  {}",
                    meta.id,
                    meta.updated_at,
                    meta.workspace.display()
                );
            }
            Ok(ExitCode(0))
        }
        SessionsCmd::Prune {
            id,
            all,
            workspace,
            older_than,
        } => {
            let spec = if all {
                PruneSpec::All
            } else if let Some(days) = older_than {
                PruneSpec::OlderThanDays(days)
            } else if let Some(ws) = workspace {
                PruneSpec::Workspace(ws)
            } else if let Some(id) = id {
                PruneSpec::Id(id)
            } else {
                return Err(AppError::PruneNeedsArg.into());
            };
            let n = session_store::prune(spec)?;
            println!("{}", i18n::tr(Msg::PruneCli, &[("n", &n.to_string())]));
            Ok(ExitCode(0))
        }
    }
}
