use crate::cli::args::OutputFormat;
use anyhow::Result;
use std::path::PathBuf;
use tester_core::error::ExitCode;

pub(crate) fn cmd_doctor(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let report = tester_pipeline::doctor::run_doctor(path)?;
    match format {
        OutputFormat::Json => println!("{}", report.render_json()),
        OutputFormat::Text => print!("{}", report.render_text()),
    }
    Ok(if report.all_ok() {
        ExitCode::OK
    } else {
        ExitCode::ERROR
    })
}
