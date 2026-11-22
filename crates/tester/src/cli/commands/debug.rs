use crate::cli::args::OutputFormat;
use crate::cli::helpers::{
    apply_config_flags, emit_report, load_config, need_llm_with_model, parse_kind, prepare_project,
};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tester_core::error::ExitCode;
use tester_pipeline::report::exit_for;

pub(crate) fn cmd_debug(
    path: &PathBuf,
    bug: Option<String>,
    bug_file: Option<&Path>,
    stdin: bool,
    reproduce_only: bool,
    dry_run: bool,
    update: bool,
    kind: &str,
    max_items: usize,
    format: OutputFormat,
    model: Option<&str>,
) -> Result<ExitCode> {
    let bug_text = tester_pipeline::debug::read_bug_input(bug, bug_file, stdin)?;
    let mut profile = prepare_project(path)?;
    let config = load_config()?;
    apply_config_flags(&mut profile, &config);
    let llm = need_llm_with_model(&config, model)?;
    let opts = tester_pipeline::debug::DebugOptions {
        bug_text,
        reproduce_only,
        write: !dry_run,
        update,
        kind: parse_kind(kind)?,
        max_items,
    };
    let mut report = tester_pipeline::debug::run_debug(llm.as_ref(), &profile, &config, &opts)?;
    tester_pipeline::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}
