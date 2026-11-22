use crate::{AssistantToolCall, ChatMessage, ChatResult, FunctionCall, Llm, ToolSpec, Usage};
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, Read};
use std::time::Duration;
use tester_core::config::LlmConfig;
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};
use tracing::{debug, info, warn};

pub(crate) struct CompatClient {
    http: reqwest::blocking::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    max_tokens: u32,
}

impl CompatClient {
    pub fn new(config: &LlmConfig, api_key: Option<String>) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        let key = api_key.or_else(|| config.api_key().map(str::to_string));
        Ok(Self {
            http,
            base_url: config.base_url().to_string(),
            model: config.model.clone(),
            api_key: key,
            max_tokens: config.max_tokens,
        })
    }

    fn auth(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        if let Some(key) = &self.api_key {
            req.bearer_auth(key)
        } else {
            req
        }
    }

    fn post_chat(
        &self,
        body: serde_json::Value,
        stream: bool,
    ) -> Result<reqwest::blocking::Response> {
        let url = format!("{}/chat/completions", self.base_url);
        info!(model = %self.model, url = %url, stream, "запрос к llm");
        debug!(body = %body, "тело llm");

        let mut last_err = None;
        for attempt in 1..=4 {
            let req = self.auth(self.http.post(&url).json(&body));
            match req.send() {
                Ok(resp) => {
                    let status = resp.status();
                    let code = status.as_u16();
                    if code == 401 || code == 403 {
                        let text = resp.text().unwrap_or_default();
                        bail!(
                            "{}",
                            i18n::tr(
                                Msg::ErrLlmAccessDenied,
                                &[("status", &status.to_string()), ("detail", &text),],
                            )
                        );
                    }
                    if code == 429 || status.is_server_error() {
                        warn!(%status, attempt, "повторяемый статус llm");
                        last_err = Some(anyhow!(
                            "{}",
                            i18n::tr(Msg::ErrLlmHttpStatus, &[("status", &status.to_string())])
                        ));
                        let backoff = 200u64.saturating_mul(1u64 << (attempt - 1).min(4));
                        std::thread::sleep(Duration::from_millis(backoff));
                        continue;
                    }
                    if !status.is_success() {
                        let text = resp.text().unwrap_or_default();
                        bail!(
                            "{}",
                            i18n::tr(
                                Msg::ErrLlmHttp,
                                &[("status", &status.to_string()), ("detail", &text)],
                            )
                        );
                    }
                    return Ok(resp);
                }
                Err(err) => {
                    warn!(attempt, error = %err, "повторяемая сетевая ошибка llm");
                    last_err = Some(err.into());
                    let backoff = 200u64.saturating_mul(1u64 << (attempt - 1).min(4));
                    std::thread::sleep(Duration::from_millis(backoff));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::Error::new(AppError::LlmUnavailable)))
    }
}

#[derive(Serialize)]
struct ApiTool<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    function: ApiFunction<'a>,
}

#[derive(Serialize)]
struct ApiFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<UsageResp>,
}

#[derive(Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Deserialize)]
struct RespMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<AssistantToolCall>,
}

#[derive(Deserialize)]
struct UsageResp {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

fn build_body(
    model: &str,
    max_tokens: u32,
    messages: &[ChatMessage],
    tools: &[ToolSpec],
    stream: bool,
) -> serde_json::Value {
    let mut body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": 0.1,
        "stream": stream,
    });
    if !tools.is_empty() {
        let api_tools: Vec<ApiTool> = tools
            .iter()
            .map(|t| ApiTool {
                kind: "function",
                function: ApiFunction {
                    name: &t.name,
                    description: &t.description,
                    parameters: &t.parameters,
                },
            })
            .collect();
        body["tools"] = serde_json::to_value(api_tools).unwrap_or(json!([]));
        body["tool_choice"] = json!("auto");
    }
    if stream {
        body["stream_options"] = json!({ "include_usage": true });
    }
    body
}

fn parse_non_stream(resp: reqwest::blocking::Response) -> Result<ChatResult> {
    let parsed: ChatResponse = resp.json().context("разбор ответа llm")?;
    let choice = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!(i18n::t(Msg::ErrLlmEmptyResponse)))?;
    let usage = parsed.usage.map(|u| Usage {
        prompt_tokens: u.prompt_tokens.unwrap_or(0),
        completion_tokens: u.completion_tokens.unwrap_or(0),
    });
    Ok(ChatResult {
        content: choice.message.content,
        tool_calls: choice.message.tool_calls,
        usage,
    })
}

fn parse_sse_stream(
    resp: reqwest::blocking::Response,
    on_delta: Option<&dyn Fn(&str)>,
) -> Result<ChatResult> {
    let mut reader = BufReader::new(resp);
    let mut content = String::new();
    let mut tool_calls: Vec<AssistantToolCall> = Vec::new();
    let mut usage = None;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(':') {
            continue;
        }
        let Some(data) = trimmed.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let chunk: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(u) = chunk.get("usage") {
            usage = Some(Usage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            });
        }
        let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) else {
            continue;
        };
        let Some(choice) = choices.first() else {
            continue;
        };
        if let Some(delta) = choice.get("delta") {
            if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                content.push_str(text);
                if let Some(cb) = on_delta {
                    cb(text);
                }
            }
            if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                for tc in tcs {
                    let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    while tool_calls.len() <= idx {
                        tool_calls.push(AssistantToolCall {
                            id: String::new(),
                            kind: "function".into(),
                            function: FunctionCall {
                                name: String::new(),
                                arguments: String::new(),
                            },
                        });
                    }
                    let slot = &mut tool_calls[idx];
                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                        slot.id = id.to_string();
                    }
                    if let Some(func) = tc.get("function") {
                        if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                            slot.function.name.push_str(name);
                        }
                        if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                            slot.function.arguments.push_str(args);
                        }
                    }
                }
            }
        }
    }

    let mut rest = Vec::new();
    let _ = reader.read_to_end(&mut rest);

    Ok(ChatResult {
        content: if content.is_empty() {
            None
        } else {
            Some(content)
        },
        tool_calls: tool_calls
            .into_iter()
            .filter(|t| !t.function.name.is_empty())
            .collect(),
        usage,
    })
}

impl Llm for CompatClient {
    fn complete(&self, system: &str, user: &str) -> Result<String> {
        let messages = vec![ChatMessage::system(system), ChatMessage::user(user)];
        let result = self.chat(&messages, &[], None)?;
        result
            .content
            .ok_or_else(|| anyhow!(i18n::t(Msg::ErrLlmEmptyResponse)))
    }

    fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        on_delta: Option<&dyn Fn(&str)>,
    ) -> Result<ChatResult> {
        let want_stream = on_delta.is_some();
        let body = build_body(&self.model, self.max_tokens, messages, tools, want_stream);
        let resp = self.post_chat(body, want_stream)?;
        if want_stream {
            parse_sse_stream(resp, on_delta)
        } else {
            parse_non_stream(resp)
        }
    }
}
