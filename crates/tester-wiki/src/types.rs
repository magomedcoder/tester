use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiKind {
    Mediawiki,
    Markdown,
    HtmlCrawl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSource {
    pub kind: WikiKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: String,
    pub title: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub body: String,
    #[serde(default)]
    pub links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiIndex {
    pub pages: Vec<WikiPageMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPageMeta {
    pub id: String,
    pub title: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default)]
    pub links: Vec<String>,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiAnalysis {
    pub versions: Vec<String>,
    pub entities: Vec<WikiEntity>,
    pub page_classes: Vec<PageClass>,
    pub hub_pages: Vec<String>,
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    #[serde(default)]
    pub graph_edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub page_id: String,
    pub title: String,
    pub version: String,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntity {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub source_pages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageClass {
    pub page_id: String,
    pub class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiScenario {
    pub id: String,
    pub version: String,
    pub title: String,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub steps: Vec<ScenarioStep>,
    #[serde(default)]
    pub source_pages: Vec<String>,
    #[serde(default = "default_ui_kind")]
    pub kind: String,
    #[serde(default = "default_coverage")]
    pub coverage: String,
}

fn default_coverage() -> String {
    "uncovered".into()
}

fn default_ui_kind() -> String {
    "ui".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiScenarios {
    pub scenarios: Vec<WikiScenario>,
}
