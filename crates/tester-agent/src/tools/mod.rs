mod analysis;
mod domain;
mod fs;
mod git;
mod security;
mod shell;
mod wiki;

use crate::AgentContext;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::sync::Arc;
use tester_core::i18n::{self, Msg};
use tester_providers::ToolSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    ReadOnly,
    Write,
    Shell,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn kind(&self) -> ToolKind;
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String>;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self { tools }
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.iter().map(|t| t.spec()).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }
}

pub fn default_registry(ctx: &AgentContext) -> ToolRegistry {
    let _ = ctx;
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(fs::ReadFile),
        Arc::new(fs::ListDir),
        Arc::new(fs::GlobFiles),
        Arc::new(fs::GrepFiles),
        Arc::new(fs::WriteFile),
        Arc::new(fs::ApplyPatch),
        Arc::new(git::GitStatus),
        Arc::new(git::GitDiff),
        Arc::new(shell::RunArgv),
        Arc::new(domain::ScanProject),
        Arc::new(domain::BuildPlan),
        Arc::new(domain::WriteTest),
        Arc::new(domain::RunTests),
        Arc::new(domain::RepairTest),
        Arc::new(domain::DebugFailure),
        Arc::new(analysis::ProjectMapTool),
        Arc::new(analysis::OutlineFile),
        Arc::new(analysis::FindSymbol),
        Arc::new(analysis::FindDefinition),
        Arc::new(analysis::FindReferences),
        Arc::new(analysis::GraphLookup),
        Arc::new(analysis::GraphCallers),
        Arc::new(analysis::GraphCallees),
        Arc::new(analysis::GraphPath),
        Arc::new(analysis::GraphNeighbors),
        Arc::new(analysis::SemanticSearch),
        Arc::new(analysis::ExpandContext),
        Arc::new(analysis::IndexStatusTool),
        Arc::new(analysis::ChangedSymbols),
        Arc::new(security::SecurityScan),
        Arc::new(security::SecurityExplain),
        Arc::new(security::SecurityPatch),
        Arc::new(wiki::WikiSync),
        Arc::new(wiki::WikiSearch),
        Arc::new(wiki::WikiScenarioList),
    ];
    if ctx.config.run.allow_docker {
        tools.push(Arc::new(shell::DockerInfo));
    }
    ToolRegistry::new(tools)
}

pub(crate) fn arg_str(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!(i18n::tr(Msg::ErrToolArgRequired, &[("key", key)])))
}

pub(crate) fn arg_str_opt(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub(crate) fn arg_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}\n... truncated ({max} chars)")
    }
}
