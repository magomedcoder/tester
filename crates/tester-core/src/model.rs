use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    #[serde(rename = "typescript")]
    TypeScript,
    #[serde(rename = "javascript")]
    JavaScript,
    Go,
    Python,
    Dart,
    Flutter,
    Kotlin,
    Php,
    Cpp,
    Unknown,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::Python => "python",
            Self::Dart => "dart",
            Self::Flutter => "flutter",
            Self::Kotlin => "kotlin",
            Self::Php => "php",
            Self::Cpp => "cpp",
            Self::Unknown => "unknown",
        }
    }

    pub fn supported(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    Unit,
    Integration,
    Ui,
    Contract,
    All,
}

impl TestKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Integration => "integration",
            Self::Ui => "ui",
            Self::Contract => "contract",
            Self::All => "all",
        }
    }

    pub fn expand(self) -> Vec<TestKind> {
        match self {
            Self::All => vec![Self::Unit, Self::Integration, Self::Ui, Self::Contract],
            other => vec![other],
        }
    }
}

impl std::str::FromStr for TestKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unit" => Ok(Self::Unit),
            "integration" => Ok(Self::Integration),
            "ui" => Ok(Self::Ui),
            "contract" => Ok(Self::Contract),
            "all" => Ok(Self::All),
            other => Err(format!("неизвестный вид теста: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStyle {
    InlineCfgTest,
    SeparateTestsDir,
    NeighborFile,
    Playwright,
    GoTestFile,
    Pytest,
    DartTest,
    JUnit,
    PhpUnit,
    GTest,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub unit: bool,
    pub integration: bool,
    pub contract: bool,
    pub ui: bool,
}

impl Capabilities {
    pub fn list(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.unit {
            out.push("unit");
        }
        if self.integration {
            out.push("integration");
        }
        if self.contract {
            out.push("contract");
        }
        if self.ui {
            out.push("ui");
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfile {
    pub root: PathBuf,
    pub language: Language,
    pub package_name: String,
    pub edition: Option<String>,
    pub supported: bool,
    pub test_command: String,
    pub test_style: TestStyle,
    pub src_dir: Option<PathBuf>,
    pub tests_dir: Option<PathBuf>,
    pub has_existing_tests: bool,
    pub unit_framework: Option<String>,
    pub ui_framework: Option<String>,
    pub ui_ready: bool,
    pub capabilities: Capabilities,
    pub notes: Vec<String>,
    #[serde(default)]
    pub android_opt_in: bool,
    #[serde(default)]
    pub package_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: String,
    pub body: String,
    pub is_public: bool,
    pub uses_io: bool,
    pub branch_score: u32,

    #[serde(default)]
    pub qualname: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub docstring: String,
    #[serde(default)]
    pub start_byte: usize,
}

impl Symbol {
    pub fn finalize(mut self) -> Self {
        if self.qualname.is_empty() {
            if let Some(parent) = &self.parent {
                self.qualname = format!("{parent}::{}", self.name);
            } else {
                self.qualname = self.name.clone();
            }
        }
        if self.visibility.is_empty() {
            self.visibility = if self.is_public {
                "pub".into()
            } else {
                "private".into()
            };
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInventoryItem {
    pub path: PathBuf,
    pub names: Vec<String>,
    pub mentioned_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestInventory {
    pub items: Vec<TestInventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gap {
    pub symbol: Symbol,
    pub kind: TestKind,
    pub reason: String,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedCase {
    pub name: String,
    pub description: String,
    pub input: String,
    pub expectation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub gap: Gap,
    pub kind: TestKind,
    pub target_path: PathBuf,
    pub cases: Vec<PlannedCase>,
    #[serde(default)]
    pub append_to_existing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bug_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlan {
    pub items: Vec<PlanItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub path: PathBuf,
    pub content: String,
    pub kind: TestKind,
    pub symbol_name: String,
    pub filter_name: String,
    #[serde(default)]
    pub append: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Planned,
    DryRun,
    Written,
    Passed,
    Unfixed,
    Skipped,
    FailedPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportItem {
    pub symbol: String,
    pub kind: TestKind,
    pub file: PathBuf,
    pub attempts: u32,
    pub status: ItemStatus,
    pub command: String,
    pub message: String,
    pub trace_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_pass: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assert_density: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usefulness: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_first_pass_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usefulness_avg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunReport {
    pub items: Vec<ReportItem>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub metrics: RunMetrics,
}
