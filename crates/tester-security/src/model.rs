use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "info" => Some(Self::Info),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" | "crit" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn meets_threshold(self, fail_on: Self) -> bool {
        self >= fail_on
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStrategy {
    Sanitize,
    Authz,
    Remove,
    Config,
    Deps,
    Advise,
}

impl PatchStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sanitize => "sanitize",
            Self::Authz => "authz",
            Self::Remove => "remove",
            Self::Config => "config",
            Self::Deps => "deps",
            Self::Advise => "advise",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixHint {
    pub summary: String,
    pub strategy: PatchStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    pub evidence: String,
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cwe: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owasp: Vec<String>,
    pub fix: FixHint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub root: PathBuf,
    pub rule_pack_version: String,
    pub deep: bool,
    pub findings: Vec<Finding>,
    pub notes: Vec<String>,
}

impl SecurityReport {
    pub fn counts_by_severity(&self) -> [(Severity, usize); 5] {
        let mut out = [
            (Severity::Critical, 0),
            (Severity::High, 0),
            (Severity::Medium, 0),
            (Severity::Low, 0),
            (Severity::Info, 0),
        ];
        for f in &self.findings {
            match f.severity {
                Severity::Critical => out[0].1 += 1,
                Severity::High => out[1].1 += 1,
                Severity::Medium => out[2].1 += 1,
                Severity::Low => out[3].1 += 1,
                Severity::Info => out[4].1 += 1,
            }
        }
        out
    }

    pub fn above(&self, threshold: Severity) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(move |f| f.severity.meets_threshold(threshold))
    }

    pub fn find(&self, id: &str) -> Option<&Finding> {
        self.findings.iter().find(|f| f.id == id || f.rule_id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchDraft {
    pub finding_id: String,
    pub path: PathBuf,
    pub dry_run: bool,
    pub applied: bool,
    pub diff: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub deep: bool,
    pub fetch_advisories: bool,
    pub severities: Vec<Severity>,
    pub disabled_rules: Vec<String>,
    pub max_files: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            deep: false,
            fetch_advisories: false,
            severities: vec![
                Severity::Critical,
                Severity::High,
                Severity::Medium,
                Severity::Low,
                Severity::Info,
            ],
            disabled_rules: Vec::new(),
            max_files: 4000,
        }
    }
}

impl ScanOptions {
    pub fn from_config(
        cfg: &tester_core::config::SecurityConfig,
        deep_override: Option<bool>,
    ) -> Self {
        let mut severities = cfg
            .severities
            .iter()
            .filter_map(|s| Severity::parse(s))
            .collect::<Vec<_>>();
        if severities.is_empty() {
            severities = Self::default().severities;
        }
        Self {
            deep: deep_override.unwrap_or(cfg.deep),
            fetch_advisories: cfg.fetch_advisories,
            severities,
            disabled_rules: cfg.disabled_rules.clone(),
            max_files: cfg.max_files.max(100),
        }
    }
}
