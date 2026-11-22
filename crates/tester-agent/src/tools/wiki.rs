use super::{Tool, ToolKind, arg_str, arg_str_opt, truncate};
use crate::AgentContext;
use anyhow::Result;
use serde_json::{Value, json};
use tester_core::i18n::{self, Msg};
use tester_wiki::{SyncOpts, analyze, load_index, load_page_body, load_scenarios, sync};

pub struct WikiSync;
impl Tool for WikiSync {
    fn name(&self) -> &str {
        "wiki_sync"
    }
    fn description(&self) -> &str {
        i18n::t(Msg::ToolWikiSync)
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {"type": "string"},
                "path": {"type": "string", "description": "local markdown docs root"},
                "max_pages": {"type": "integer"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let max_pages = args
            .get("max_pages")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as usize;
        let (source, index) = sync(
            &ctx.profile.root,
            SyncOpts {
                url: arg_str_opt(args, "url"),
                path: arg_str_opt(args, "path").map(std::path::PathBuf::from),
                max_pages,
            },
        )?;
        ctx.audit.record(
            "wiki_sync",
            &format!("{:?} pages={}", source.kind, index.pages.len()),
            "ok",
        );
        Ok(serde_json::json!({
            "kind": source.kind,
            "url": source.url,
            "path": source.path,
            "pages": index.pages.len(),
        })
        .to_string())
    }
}

pub struct WikiSearch;
impl Tool for WikiSearch {
    fn name(&self) -> &str {
        "wiki_search"
    }
    fn description(&self) -> &str {
        i18n::t(Msg::ToolWikiSearch)
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer"}
            },
            "required": ["query"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let query = arg_str(args, "query")?.to_ascii_lowercase();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let index = load_index(&ctx.profile.root)?;
        let mut hits = Vec::new();
        for meta in &index.pages {
            let title_l = meta.title.to_ascii_lowercase();
            let body = load_page_body(&ctx.profile.root, meta)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if title_l.contains(&query) || body.contains(&query) {
                hits.push(serde_json::json!({
                    "id": meta.id,
                    "title": meta.title,
                    "version": meta.version,
                    "url": meta.url,
                }));
                if hits.len() >= limit.max(1) {
                    break;
                }
            }
        }
        Ok(serde_json::to_string_pretty(&hits)?)
    }
}

pub struct WikiScenarioList;
impl Tool for WikiScenarioList {
    fn name(&self) -> &str {
        "wiki_scenario_list"
    }
    fn description(&self) -> &str {
        i18n::t(Msg::ToolWikiScenarioList)
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "version": {"type": "string"},
                "uncovered_only": {"type": "boolean"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        if load_scenarios(&ctx.profile.root)?.scenarios.is_empty() {
            let _ = analyze(&ctx.profile.root, None)?;
        }
        let version = arg_str_opt(args, "version");
        let uncovered_only = args
            .get("uncovered_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let scenarios = load_scenarios(&ctx.profile.root)?;
        let list: Vec<_> = scenarios
            .scenarios
            .iter()
            .filter(|s| {
                if let Some(v) = &version
                    && s.version != *v
                    && s.version != "default"
                {
                    return false;
                }
                if uncovered_only && s.coverage == "covered" {
                    return false;
                }
                true
            })
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "title": s.title,
                    "version": s.version,
                    "coverage": s.coverage,
                    "steps": s.steps.len(),
                })
            })
            .collect();
        Ok(truncate(&serde_json::to_string_pretty(&list)?, 12_000))
    }
}
