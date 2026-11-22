use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::time::Duration;
use tester_core::config::LlmConfig;
use tester_core::error::AppError;
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelEntry {
    id: String,
}

pub fn list_models(config: &LlmConfig) -> Result<Vec<String>> {
    let base = config.base_url.trim_end_matches('/');
    let url = format!("{base}/models");
    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs.clamp(3, 15)))
        .build()
        .context("http client")?;
    let mut req = http.get(&url);
    if let Some(key) = &config.api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }
    let resp = req.send().with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrLlmHttp,
                &[
                    ("status", &status.to_string()),
                    ("detail", &body.chars().take(200).collect::<String>()),
                ],
            )
        );
    }
    let parsed: ModelsResponse = resp.json().context("parse /models json")?;
    let mut ids: Vec<String> = parsed
        .data
        .into_iter()
        .map(|m| m.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

pub fn ensure_llm_config(mut config: LlmConfig) -> Result<LlmConfig> {
    if !config.model.trim().is_empty() {
        return Ok(config);
    }
    let models = list_models(&config)?;
    match models.as_slice() {
        [] => Err(AppError::NoModels.into()),
        [one] => {
            config.model = one.clone();
            Ok(config)
        }
        _ => Err(AppError::ModelNeedsSelection {
            count: models.len(),
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_keeps_explicit_model() {
        let cfg = LlmConfig {
            base_url: "http://127.0.0.1:9/v1".into(),
            model: "explicit".into(),
            api_key: None,
            max_tokens: 100,
            timeout_secs: 1,
        };
        let out = ensure_llm_config(cfg).unwrap();
        assert_eq!(out.model, "explicit");
    }
}
