use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub target: TargetConfig,
    pub tests: TestsConfig,
    pub run: RunConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_timeout")]
    pub timeout_secs: u64,
}

fn default_base_url() -> String {
    "http://127.0.0.1:11434/v1".into()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_llm_timeout() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestsConfig {
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_ui_framework")]
    pub framework: String,
    #[serde(default = "default_ui_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub start: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            framework: default_ui_framework(),
            base_url: default_ui_base_url(),
            start: None,
        }
    }
}

fn default_ui_framework() -> String {
    "playwright".into()
}

fn default_ui_base_url() -> String {
    "http://127.0.0.1:3000".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    #[serde(default = "default_repair_attempts")]
    pub repair_attempts: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub allow_docker: bool,
    #[serde(default = "default_max_context")]
    pub max_context_chars: usize,
    #[serde(default)]
    pub android_opt_in: bool,
    #[serde(default = "default_max_agent_turns")]
    pub max_agent_turns: u32,
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: u32,
    #[serde(default = "default_agent_deadline")]
    pub agent_deadline_secs: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    #[serde(default = "default_max_items")]
    pub max_items: u32,
    #[serde(default)]
    pub agent_auto_approve: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            repair_attempts: default_repair_attempts(),
            timeout_secs: default_timeout(),
            allow_docker: false,
            max_context_chars: default_max_context(),
            android_opt_in: false,
            max_agent_turns: default_max_agent_turns(),
            max_tool_calls: default_max_tool_calls(),
            agent_deadline_secs: default_agent_deadline(),
            concurrency: default_concurrency(),
            max_items: default_max_items(),
            agent_auto_approve: false,
        }
    }
}

fn default_concurrency() -> u32 {
    1
}

fn default_max_items() -> u32 {
    50
}

fn default_max_agent_turns() -> u32 {
    20
}

fn default_max_tool_calls() -> u32 {
    40
}

fn default_agent_deadline() -> u64 {
    600
}

fn default_repair_attempts() -> u32 {
    3
}

fn default_timeout() -> u64 {
    120
}

fn default_max_context() -> usize {
    24_000
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    llm: Option<LlmConfig>,
    target: Option<RawTarget>,
    tests: Option<TestsConfig>,
    run: Option<RunConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTarget {
    #[allow(dead_code)]
    root: Option<PathBuf>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let file = root.join("tester.toml");
        let raw = if file.exists() {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("чтение {}", file.display()))?;
            toml::from_str(&text).with_context(|| format!("разбор {}", file.display()))?
        } else {
            RawConfig::default()
        };

        let _ = raw.target;

        Ok(Config {
            llm: raw.llm.unwrap_or_else(|| LlmConfig {
                base_url: default_base_url(),
                model: String::new(),
                api_key: None,
                max_tokens: default_max_tokens(),
                timeout_secs: default_llm_timeout(),
            }),
            target: TargetConfig { root },
            tests: raw.tests.unwrap_or_default(),
            run: raw.run.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_defaults_without_file() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert_eq!(cfg.run.repair_attempts, 3);
        assert_eq!(cfg.tests.ui.framework, "playwright");
    }

    #[test]
    fn loads_tester_toml() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("tester.toml")).unwrap();
        write!(
            f,
            r#"
[llm]
model = "demo-model"
api_key = "sk-test"
[run]
repair_attempts = 2
android_opt_in = true
"#
        )
        .unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert_eq!(cfg.llm.model, "demo-model");
        assert_eq!(cfg.llm.api_key.as_deref(), Some("sk-test"));
        assert_eq!(cfg.run.repair_attempts, 2);
        assert!(cfg.run.android_opt_in);
    }
}
