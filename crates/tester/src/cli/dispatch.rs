use crate::cli::args::*;
use crate::cli::commands;
use crate::cli::helpers::{init_tracing, install_ctrlc};
use anyhow::Result;
use clap::Parser;
use std::process::ExitCode as StdExitCode;
use tester_core::error::{ExitCode, exit_from_error};
use tester_core::i18n::{self, Msg};

pub fn run() -> StdExitCode {
    let cli = Cli::parse();
    tester_core::i18n::init_from_env(cli.locale.as_deref());
    let for_tui = cli.command.is_none() && !cli.plain;
    init_tracing(cli.debug, for_tui);
    install_ctrlc();
    match run_cli(cli) {
        Ok(code) => code.into(),
        Err(err) => {
            eprintln!("{}{err:#}", i18n::t(Msg::ErrorPrefix));
            exit_from_error(&err).into()
        }
    }
}

fn run_cli(cli: Cli) -> Result<ExitCode> {
    let format = cli.format;
    let Some(command) = cli.command else {
        return commands::agent::cmd_tui(cli.path, cli.dry_run, cli.session, cli.plain);
    };
    match command {
        Commands::Test(cmd) => commands::test::dispatch_test(cmd, format),
        Commands::Agent {
            path,
            prompt,
            dry_run,
            resume,
            stream,
            no_stream,
        } => commands::agent::cmd_agent(&path, &prompt, dry_run, resume, stream && !no_stream),
        Commands::Chat {
            path,
            dry_run,
            stream,
            no_stream,
        } => commands::agent::cmd_chat(&path, dry_run, stream && !no_stream),
        Commands::Sessions { action } => commands::sessions::cmd_sessions(action),
        Commands::Debug {
            path,
            bug,
            bug_file,
            stdin,
            reproduce_only,
            dry_run,
            update,
            kind,
            max_items,
        } => commands::debug::cmd_debug(
            &path,
            bug,
            bug_file.as_deref(),
            stdin,
            reproduce_only,
            dry_run,
            update,
            &kind,
            max_items,
            format,
        ),
        Commands::Doctor { path } => commands::doctor::cmd_doctor(&path, format),
        Commands::Ui(UiCommands::Init { path }) => commands::ui::cmd_ui_init(&path, format),
        Commands::Ui(UiCommands::Doctor { path }) => commands::ui::cmd_ui_doctor(&path, format),
        Commands::Security(cmd) => commands::security::dispatch_security(cmd),
    }
}
