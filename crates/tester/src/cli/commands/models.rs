use anyhow::Result;
use std::path::PathBuf;
use tester_core::config::Config;
use tester_core::error::ExitCode;
use tester_core::i18n::{self, Msg};
use tester_llm::list_models;

pub(crate) fn cmd_models(path: Option<PathBuf>) -> Result<ExitCode> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    let config = Config::load(&root)?;
    let models = list_models(&config.llm)?;
    if models.is_empty() {
        eprintln!("{}", i18n::t(Msg::ErrNoModels));
        return Ok(ExitCode::LLM_UNAVAILABLE);
    }
    let current = config.llm.model.trim();
    for id in &models {
        if !current.is_empty() && id == current {
            println!("{id} *");
        } else {
            println!("{id}");
        }
    }
    Ok(ExitCode::OK)
}
