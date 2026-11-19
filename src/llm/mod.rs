mod compat;
#[cfg(test)]
mod compat_tests;
mod types;

pub use compat::CompatClient;
pub use types::{AssistantToolCall, ChatMessage, ChatResult, FunctionCall, ToolSpec, Usage};

use crate::config::LlmConfig;
use anyhow::Result;
use std::sync::Arc;

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
    Ok(Arc::new(CompatClient::new(config, config.api_key.clone())?))
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
