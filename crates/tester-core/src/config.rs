use crate::i18n::{self, Msg};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub locale: Option<String>,
    pub llm: LlmConfig,
    pub tests: TestsConfig,
    pub run: RunConfig,
    #[serde(default)]
    pub analysis: AnalysisConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub documents: DocumentsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentsConfig {
    #[serde(default)]
    pub pdftotext: Option<String>,
    #[serde(default)]
    pub pdftoppm: Option<String>,
    #[serde(default)]
    pub tesseract: Option<String>,
    #[serde(default)]
    pub soffice: Option<String>,
    #[serde(default)]
    pub pandoc: Option<String>,
    #[serde(default = "default_ocr_lang")]
    pub ocr_lang: String,
    #[serde(default = "default_true")]
    pub cache: bool,
}

impl Default for DocumentsConfig {
    fn default() -> Self {
        Self {
            pdftotext: None,
            pdftoppm: None,
            tesseract: None,
            soffice: None,
            pandoc: None,
            ocr_lang: default_ocr_lang(),
            cache: true,
        }
    }
}

fn default_ocr_lang() -> String {
    "eng+rus".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub deep: bool,
    #[serde(default)]
    pub fetch_advisories: bool,
    #[serde(default = "default_security_severities")]
    pub severities: Vec<String>,
    #[serde(default)]
    pub auto_patch: bool,
    #[serde(default)]
    pub disabled_rules: Vec<String>,
    #[serde(default = "default_security_max_files")]
    pub max_files: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deep: false,
            fetch_advisories: false,
            severities: default_security_severities(),
            auto_patch: false,
            disabled_rules: Vec::new(),
            max_files: default_security_max_files(),
        }
    }
}

fn default_security_severities() -> Vec<String> {
    vec![
        "critical".into(),
        "high".into(),
        "medium".into(),
        "low".into(),
        "info".into(),
    ]
}

fn default_security_max_files() -> usize {
    4000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisConfig {
    #[serde(default = "default_true")]
    pub index: bool,
    #[serde(default = "default_true")]
    pub graph: bool,
    #[serde(default)]
    pub embeddings: bool,
    #[serde(default)]
    pub embeddings_model: String,
    #[serde(default = "default_max_map_tokens")]
    pub max_map_tokens: u32,
    #[serde(default = "default_semantic_top_k")]
    pub semantic_top_k: u32,
    #[serde(default = "default_grep_max_hits")]
    pub grep_max_hits: u32,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            index: true,
            graph: true,
            embeddings: false,
            embeddings_model: String::new(),
            max_map_tokens: default_max_map_tokens(),
            semantic_top_k: default_semantic_top_k(),
            grep_max_hits: default_grep_max_hits(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_map_tokens() -> u32 {
    2000
}

fn default_semantic_top_k() -> u32 {
    8
}

fn default_grep_max_hits() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmProviderConfig {
    pub id: String,
    #[serde(default = "default_provider_kind")]
    pub kind: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_provider_kind() -> String {
    "openai_compat".into()
}

fn default_providers() -> Vec<LlmProviderConfig> {
    vec![LlmProviderConfig {
        id: "llamacpp".into(),
        kind: default_provider_kind(),
        base_url: "http://127.0.0.1:8080/v1".into(),
        api_key: None,
    }]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmConfig {
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_timeout")]
    pub timeout_secs: u64,
    #[serde(skip)]
    pub providers: Vec<LlmProviderConfig>,
    #[serde(skip)]
    pub model: String,
    #[serde(skip)]
    pub active_provider_id: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            timeout_secs: default_llm_timeout(),
            providers: default_providers(),
            model: String::new(),
            active_provider_id: String::new(),
        }
    }
}

impl LlmConfig {
    pub fn ensure_providers(&mut self) {
        if self.providers.is_empty() {
            self.providers = default_providers();
        }
        if self.active_provider_id.trim().is_empty() {
            if self.providers.len() == 1 {
                self.active_provider_id = self.providers[0].id.clone();
            }
        } else if !self
            .providers
            .iter()
            .any(|p| p.id == self.active_provider_id)
        {
            self.active_provider_id = self
                .providers
                .first()
                .map(|p| p.id.clone())
                .unwrap_or_default();
        }
    }

    pub fn active_provider(&self) -> &LlmProviderConfig {
        self.providers
            .iter()
            .find(|p| p.id == self.active_provider_id)
            .or_else(|| self.providers.first())
            .expect("LlmConfig.providers must not be empty")
    }

    pub fn base_url(&self) -> &str {
        self.active_provider().base_url.trim_end_matches('/')
    }

    pub fn api_key(&self) -> Option<&str> {
        self.active_provider()
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
    }
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_llm_timeout() -> u64 {
    120
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TestsConfig {
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    #[serde(default = "default_agent_auto_approve")]
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
            agent_auto_approve: default_agent_auto_approve(),
        }
    }
}

fn default_agent_auto_approve() -> bool {
    true
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
#[serde(deny_unknown_fields)]
struct RawConfig {
    locale: Option<String>,
    #[serde(default)]
    providers: Vec<LlmProviderConfig>,
    llm: Option<LlmConfig>,
    tests: Option<TestsConfig>,
    run: Option<RunConfig>,
    analysis: Option<AnalysisConfig>,
    security: Option<SecurityConfig>,
    documents: Option<DocumentsConfig>,
}

pub fn tester_home() -> PathBuf {
    if let Ok(p) = std::env::var("TESTER_HOME") {
        return PathBuf::from(p);
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tester")
}

const GLOBAL_CONFIG_TEMPLATE: &str = r#"
locale = "en"

[[providers]]
id = "llamacpp"
kind = "openai_compat"
base_url = "http://127.0.0.1:8080/v1"
# api_key = ""

[run]
agent_auto_approve = true
allow_docker = false

[analysis]
embeddings = false

[security]
enabled = true
deep = false
fetch_advisories = false

[documents]
# pdftotext / pdftoppm / tesseract / soffice / pandoc paths optional (default: PATH)
ocr_lang = "eng+rus"
cache = true
"#;

pub(crate) fn ensure_global_config() -> Result<PathBuf> {
    let home = tester_home();
    std::fs::create_dir_all(&home).with_context(|| i18n::io_ctx(Msg::IoCreate, &home))?;
    let path = home.join("tester.toml");
    if !path.exists() {
        std::fs::write(&path, GLOBAL_CONFIG_TEMPLATE)
            .with_context(|| i18n::io_ctx(Msg::IoWrite, &path))?;
    }
    Ok(path)
}

fn llm_model_path() -> PathBuf {
    tester_home().join("model")
}

fn llm_provider_path() -> PathBuf {
    tester_home().join("provider")
}

fn load_llm_model() -> Option<String> {
    let text = std::fs::read_to_string(llm_model_path()).ok()?;
    let id = text.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

fn load_llm_provider() -> Option<String> {
    let text = std::fs::read_to_string(llm_provider_path()).ok()?;
    let id = text.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

pub fn persist_llm_provider(provider_id: &str) -> Result<PathBuf> {
    let id = provider_id.trim();
    if id.is_empty() {
        anyhow::bail!("provider id is empty");
    }
    let home = tester_home();
    std::fs::create_dir_all(&home).with_context(|| i18n::io_ctx(Msg::IoCreate, &home))?;
    let path = llm_provider_path();
    let tmp = home.join("provider.tmp");
    std::fs::write(&tmp, format!("{id}\n")).with_context(|| i18n::io_ctx(Msg::IoWrite, &tmp))?;
    std::fs::rename(&tmp, &path).with_context(|| i18n::io_ctx(Msg::IoWrite, &path))?;
    Ok(path)
}

pub fn persist_llm_model(model: &str) -> Result<PathBuf> {
    let id = model.trim();
    if id.is_empty() {
        anyhow::bail!("{}", i18n::t(Msg::ModelSelectHint));
    }
    let home = tester_home();
    std::fs::create_dir_all(&home).with_context(|| i18n::io_ctx(Msg::IoCreate, &home))?;
    let path = llm_model_path();
    let tmp = home.join("model.tmp");
    std::fs::write(&tmp, format!("{id}\n")).with_context(|| i18n::io_ctx(Msg::IoWrite, &tmp))?;
    std::fs::rename(&tmp, &path).with_context(|| i18n::io_ctx(Msg::IoWrite, &path))?;
    Ok(path)
}

fn read_toml_file(path: &Path) -> Result<Option<toml::Value>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| i18n::io_ctx(Msg::IoRead, path))?;
    let value = toml::from_str(&text).with_context(|| i18n::io_ctx(Msg::IoParse, path))?;
    Ok(Some(value))
}

fn config_from_raw(raw: RawConfig) -> Config {
    let mut llm = raw.llm.unwrap_or_default();
    llm.providers = if raw.providers.is_empty() {
        default_providers()
    } else {
        raw.providers
    };
    llm.ensure_providers();
    Config {
        locale: raw.locale,
        llm,
        tests: raw.tests.unwrap_or_default(),
        run: raw.run.unwrap_or_default(),
        analysis: raw.analysis.unwrap_or_default(),
        security: raw.security.unwrap_or_default(),
        documents: raw.documents.unwrap_or_default(),
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let global_path = ensure_global_config()?;
        let raw: RawConfig = match read_toml_file(&global_path)? {
            Some(value) => value
                .try_into()
                .with_context(|| i18n::io_ctx(Msg::IoParse, &global_path))?,
            None => RawConfig::default(),
        };

        let mut config = config_from_raw(raw);
        if let Some(model) = load_llm_model() {
            config.llm.model = model;
        }
        if let Some(provider) = load_llm_provider() {
            config.llm.active_provider_id = provider;
            config.llm.ensure_providers();
        }
        crate::i18n::apply_config_locale(config.locale.as_deref());
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_tester_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("TESTER_HOME", home) };
        let out = f();
        unsafe { std::env::remove_var("TESTER_HOME") };
        out
    }

    #[test]
    fn loads_defaults_without_file() {
        let home = tempfile::tempdir().unwrap();
        let cfg = with_tester_home(home.path(), || Config::load().unwrap());
        assert!(home.path().join("tester.toml").is_file());
        assert_eq!(cfg.run.repair_attempts, 3);
        assert!(cfg.run.agent_auto_approve);
        assert_eq!(cfg.tests.ui.framework, "playwright");
        assert_eq!(cfg.llm.active_provider().id, "llamacpp");
    }

    #[test]
    fn ensure_global_config_creates_once() {
        let home = tempfile::tempdir().unwrap();
        with_tester_home(home.path(), || {
            let path = ensure_global_config().unwrap();
            assert!(path.is_file());
            let first = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, "locale = \"ru\"\n").unwrap();
            ensure_global_config().unwrap();
            let second = std::fs::read_to_string(&path).unwrap();
            assert_eq!(second, "locale = \"ru\"\n");
            assert!(first.contains("[[providers]]"));
            assert!(!first.contains("[[llm.providers]]"));
        });
    }

    #[test]
    fn loads_global_tester_toml() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            r#"
[[providers]]
id = "cloud"
kind = "openai_compat"
base_url = "http://cloud:1/v1"
api_key = "sk-test"

[run]
repair_attempts = 2
android_opt_in = true
"#,
        )
        .unwrap();
        let cfg = with_tester_home(home.path(), || {
            persist_llm_model("demo-model").unwrap();
            persist_llm_provider("cloud").unwrap();
            Config::load().unwrap()
        });
        assert_eq!(cfg.llm.model, "demo-model");
        assert_eq!(cfg.llm.active_provider().id, "cloud");
        assert_eq!(cfg.llm.api_key(), Some("sk-test"));
        assert_eq!(cfg.run.repair_attempts, 2);
        assert!(cfg.run.android_opt_in);
        assert!(cfg.run.agent_auto_approve);
    }

    #[test]
    fn ignores_project_tester_toml() {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            r#"
[[providers]]
id = "global"
kind = "openai_compat"
base_url = "http://global:8080/v1"

[run]
repair_attempts = 9
"#,
        )
        .unwrap();
        std::fs::write(
            project.path().join("tester.toml"),
            r#"
[[providers]]
id = "project"
kind = "openai_compat"
base_url = "http://project:1/v1"

[run]
repair_attempts = 1
android_opt_in = true
"#,
        )
        .unwrap();
        let cfg = with_tester_home(home.path(), || Config::load().unwrap());
        assert_eq!(cfg.llm.active_provider().id, "global");
        assert_eq!(cfg.llm.base_url(), "http://global:8080/v1");
        assert_eq!(cfg.run.repair_attempts, 9);
        assert!(!cfg.run.android_opt_in);
    }

    #[test]
    fn rejects_unknown_toml_fields() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            r#"
[llm]
model = "from-toml"

[[llm.providers]]
id = "x"
kind = "openai_compat"
base_url = "http://x:1/v1"
"#,
        )
        .unwrap();
        let err = with_tester_home(home.path(), || Config::load().unwrap_err());
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unknown field") || msg.contains("providers") || msg.contains("model"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn rejects_unknown_top_level_keys() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            "model = \"orphan\"\n[[providers]]\nid = \"llamacpp\"\nkind = \"openai_compat\"\nbase_url = \"http://127.0.0.1:8080/v1\"\n",
        )
        .unwrap();
        let err = with_tester_home(home.path(), || Config::load().unwrap_err());
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unknown field") || msg.contains("model"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn persist_llm_model_roundtrip() {
        let home = tempfile::tempdir().unwrap();
        with_tester_home(home.path(), || {
            let path = persist_llm_model("qwen2.5").unwrap();
            assert_eq!(path, home.path().join("model"));
            assert_eq!(load_llm_model().as_deref(), Some("qwen2.5"));
            assert!(persist_llm_model("  ").is_err());
        });
    }

    #[test]
    fn agent_auto_approve_can_be_disabled() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            "[run]\nagent_auto_approve = false\n",
        )
        .unwrap();
        let cfg = with_tester_home(home.path(), || Config::load().unwrap());
        assert!(!cfg.run.agent_auto_approve);
    }

    #[test]
    fn rejects_unknown_documents_fields() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("tester.toml"),
            "[documents]\nocr_cache_dir = \"/tmp\"\n",
        )
        .unwrap();
        let err = with_tester_home(home.path(), || Config::load().unwrap_err());
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unknown field") || msg.contains("ocr_cache_dir"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn loads_documents_defaults() {
        let home = tempfile::tempdir().unwrap();
        let cfg = with_tester_home(home.path(), || Config::load().unwrap());
        assert!(cfg.documents.cache);
        assert_eq!(cfg.documents.ocr_lang, "eng+rus");
    }
}
