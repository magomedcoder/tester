#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
)]

mod compat;
#[cfg(test)]
mod compat_tests;
mod models;
mod types;

pub(crate) use compat::CompatClient;
pub use models::{ensure_llm_config, list_models};
pub use types::{AssistantToolCall, ChatMessage, ChatResult, FunctionCall, ToolSpec, Usage};

use anyhow::{Result, bail};
use std::sync::Arc;
use tester_core::config::LlmConfig;
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};

pub trait Llm: Send + Sync {
    fn complete(&self, system: &str, user: &str) -> Result<String>;

    fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        on_delta: Option<&dyn Fn(&str)>,
    ) -> Result<ChatResult>;
}

pub fn create_client(config: &LlmConfig) -> Result<Arc<dyn Llm>> {
    let cfg = ensure_llm_config(config.clone())?;
    create_client_with_model(&cfg)
}

pub fn create_client_with_model(config: &LlmConfig) -> Result<Arc<dyn Llm>> {
    if config.model.trim().is_empty() {
        bail!(AppError::ModelNeedsSelection { count: 0 });
    }
    Ok(Arc::new(CompatClient::new(config, None)?))
}

pub fn unset_model_client() -> Arc<dyn Llm> {
    Arc::new(UnsetModelLlm)
}

struct UnsetModelLlm;

impl Llm for UnsetModelLlm {
    fn complete(&self, _system: &str, _user: &str) -> Result<String> {
        bail!("{}", i18n::t(Msg::ModelSelectHint))
    }

    fn chat(
        &self,
        _messages: &[ChatMessage],
        _tools: &[ToolSpec],
        _on_delta: Option<&dyn Fn(&str)>,
    ) -> Result<ChatResult> {
        bail!("{}", i18n::t(Msg::ModelSelectHint))
    }
}

#[derive(Default)]
pub struct MockLlm {
    pub complete_response: String,
    pub chat_script: std::sync::Mutex<Vec<ChatResult>>,
}

impl MockLlm {
    pub fn with_complete(text: impl Into<String>) -> Self {
        Self {
            complete_response: text.into(),
            chat_script: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_chat(script: Vec<ChatResult>) -> Self {
        Self {
            complete_response: String::new(),
            chat_script: std::sync::Mutex::new(script),
        }
    }
}

impl Llm for MockLlm {
    fn complete(&self, _system: &str, _user: &str) -> Result<String> {
        Ok(self.complete_response.clone())
    }

    fn chat(
        &self,
        _messages: &[ChatMessage],
        _tools: &[ToolSpec],
        on_delta: Option<&dyn Fn(&str)>,
    ) -> Result<ChatResult> {
        let mut script = self.chat_script.lock().unwrap();
        let next = if script.is_empty() {
            ChatResult {
                content: Some(self.complete_response.clone()),
                tool_calls: vec![],
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                }),
            }
        } else {
            script.remove(0)
        };
        if let (Some(cb), Some(text)) = (on_delta, next.content.as_ref()) {
            cb(text);
        }
        Ok(next)
    }
}
