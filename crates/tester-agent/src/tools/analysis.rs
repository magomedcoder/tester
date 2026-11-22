use super::{Tool, ToolKind, arg_str, arg_str_opt, truncate};
use crate::AgentContext;
use anyhow::Result;
use serde_json::{Value, json};
use tester_analysis::{
    changed, context, format_hits_json, graph, map, search, semantic, truncate_hits,
};
use tester_lang::index;

fn load_symbols(ctx: &AgentContext) -> Result<Vec<tester_core::model::Symbol>> {
    if !ctx.config.analysis.index {
        return Ok(vec![]);
    }
    index::index_symbols(&ctx.profile)
}

pub struct ProjectMapTool;
impl Tool for ProjectMapTool {
    fn name(&self) -> &str {
        "project_map"
    }
    fn description(&self) -> &str {
        "Сжатая карта репозитория: языки, пакеты, entrypoints, test roots, дерево"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "depth": {"type": "integer", "description": "Глубина дерева (по умолчанию 4)"},
                "fresh": {"type": "boolean", "description": "Игнорировать кэш"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
        let fresh = args.get("fresh").and_then(|v| v.as_bool()).unwrap_or(false);
        let m = if fresh {
            map::build_project_map_fresh(&ctx.profile.root, depth)?
        } else {
            map::build_project_map(&ctx.profile.root, depth)?
        };
        let text = map::render_map_compact(&m, ctx.config.analysis.max_map_tokens);
        ctx.audit.record("project_map", ".", "ok");
        Ok(text)
    }
}

pub struct OutlineFile;
impl Tool for OutlineFile {
    fn name(&self) -> &str {
        "outline_file"
    }
    fn description(&self) -> &str {
        "Outline символов файла без полного тела"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let path = arg_str(args, "path")?;
        let symbols = load_symbols(ctx)?;
        let hits = search::outline_file(&symbols, std::path::Path::new(&path));
        ctx.audit
            .record("outline_file", &path, &format!("{} syms", hits.len()));
        Ok(context::outline_as_text(&hits))
    }
}

pub struct FindSymbol;
impl Tool for FindSymbol {
    fn name(&self) -> &str {
        "find_symbol"
    }
    fn description(&self) -> &str {
        "Поиск символа по имени/qualname (exact/prefix/fuzzy) в индексе"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "kind": {"type": "string"},
                "file": {"type": "string"},
                "module": {"type": "string"},
                "visibility": {"type": "string"},
                "include_tests": {"type": "boolean"},
                "max": {"type": "integer"}
            },
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let q = search::SymbolQuery {
            name,
            kind: arg_str_opt(args, "kind"),
            file: arg_str_opt(args, "file"),
            module: arg_str_opt(args, "module"),
            visibility: arg_str_opt(args, "visibility"),
            include_tests: args
                .get("include_tests")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            max: args.get("max").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        };
        let symbols = load_symbols(ctx)?;
        let out = search::render_symbol_search(&symbols, &q);
        ctx.audit.record("find_symbol", &q.name, "ok");
        Ok(out)
    }
}

pub struct FindDefinition;
impl Tool for FindDefinition {
    fn name(&self) -> &str {
        "find_definition"
    }
    fn description(&self) -> &str {
        "Локация definition символа (+ preview)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "preview": {"type": "boolean"}
            },
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let preview = args
            .get("preview")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let symbols = load_symbols(ctx)?;
        let Some(mut hit) = search::find_definition(&symbols, &name) else {
            ctx.audit.record("find_definition", &name, "miss");
            return Ok(format!(
                r#"{{"hits":[],"count":0,"note":"not found: {name}"}}"#
            ));
        };
        if preview {
            if let Ok(snip) = context::read_range(
                &ctx.profile.root,
                &hit.path.to_string_lossy(),
                hit.start_line,
                hit.end_line.min(hit.start_line + 40),
            ) {
                hit = hit.with_snippet(snip);
            }
        }
        ctx.audit.record("find_definition", &name, "ok");
        Ok(format_hits_json(std::slice::from_ref(&hit), 0))
    }
}

pub struct FindReferences;
impl Tool for FindReferences {
    fn name(&self) -> &str {
        "find_references"
    }
    fn description(&self) -> &str {
        "Usages символа (graph callers + text fallback)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "max": {"type": "integer"}
            },
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(40) as usize;
        let symbols = load_symbols(ctx)?;
        let mut hits = Vec::new();
        if ctx.config.analysis.graph {
            let g = graph::ensure_graph(&ctx.profile.root, &symbols, true)?;
            hits.extend(graph::callers(&g, &name, 1));
        }
        if hits.is_empty() {
            hits = search::find_references_text(&ctx.profile.root, &symbols, &name, max);
        }
        let (shown, trunc) = truncate_hits(&hits, max);
        ctx.audit
            .record("find_references", &name, &format!("{} hits", shown.len()));
        Ok(format_hits_json(&shown, trunc))
    }
}

pub struct GraphLookup;
impl Tool for GraphLookup {
    fn name(&self) -> &str {
        "graph_lookup"
    }
    fn description(&self) -> &str {
        "Узел графа кода по имени/qualname"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let symbols = load_symbols(ctx)?;
        let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
        let hits = graph::lookup(&g, &name);
        let (shown, trunc) = truncate_hits(&hits, 30);
        ctx.audit.record("graph_lookup", &name, "ok");
        Ok(format_hits_json(&shown, trunc))
    }
}

pub struct GraphCallers;
impl Tool for GraphCallers {
    fn name(&self) -> &str {
        "graph_callers"
    }
    fn description(&self) -> &str {
        "Кто вызывает символ (depth 1-2)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "depth": {"type": "integer"}
            },
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let symbols = load_symbols(ctx)?;
        let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
        let hits = graph::callers(&g, &name, depth.min(2));
        let (shown, trunc) = truncate_hits(&hits, 40);
        ctx.audit.record("graph_callers", &name, "ok");
        Ok(format_hits_json(&shown, trunc))
    }
}

pub struct GraphCallees;
impl Tool for GraphCallees {
    fn name(&self) -> &str {
        "graph_callees"
    }
    fn description(&self) -> &str {
        "Кого вызывает символ (depth 1-2)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "depth": {"type": "integer"}
            },
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let symbols = load_symbols(ctx)?;
        let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
        let hits = graph::callees(&g, &name, depth.min(2));
        let (shown, trunc) = truncate_hits(&hits, 40);
        ctx.audit.record("graph_callees", &name, "ok");
        Ok(format_hits_json(&shown, trunc))
    }
}

pub struct GraphPath;
impl Tool for GraphPath {
    fn name(&self) -> &str {
        "graph_path"
    }
    fn description(&self) -> &str {
        "Кратчайший путь A -> B в графе кода"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "from": {"type": "string"},
                "to": {"type": "string"},
                "limit": {"type": "integer"}
            },
            "required": ["from", "to"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let from = arg_str(args, "from")?;
        let to = arg_str(args, "to")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(64) as usize;
        let symbols = load_symbols(ctx)?;
        let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
        let path = graph::shortest_path(&g, &from, &to, limit);
        ctx.audit
            .record("graph_path", &format!("{from}->{to}"), "ok");
        Ok(serde_json::to_string_pretty(
            &json!({ "path": path, "len": path.len() }),
        )?)
    }
}

pub struct GraphNeighbors;
impl Tool for GraphNeighbors {
    fn name(&self) -> &str {
        "graph_neighbors"
    }
    fn description(&self) -> &str {
        "Локальный контекст графа вокруг символа (callers+callees)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let name = arg_str(args, "name")?;
        let symbols = load_symbols(ctx)?;
        let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
        let hits = graph::neighbors(&g, &name);
        let (shown, trunc) = truncate_hits(&hits, 40);
        ctx.audit.record("graph_neighbors", &name, "ok");
        Ok(format_hits_json(&shown, trunc))
    }
}

pub struct SemanticSearch;
impl Tool for SemanticSearch {
    fn name(&self) -> &str {
        "semantic_search"
    }
    fn description(&self) -> &str {
        "Смысловой поиск по embeddings (opt-in [analysis] embeddings=true)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "rebuild": {"type": "boolean"}
            },
            "required": ["query"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let query = arg_str(args, "query")?;
        let rebuild = args
            .get("rebuild")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let symbols = load_symbols(ctx)?;
        if rebuild {
            let _ = semantic::build_embeddings(
                &ctx.profile.root,
                &symbols,
                &ctx.config.llm,
                &ctx.config.analysis,
            )?;
        }
        let out = semantic::semantic_search(
            &ctx.profile.root,
            &query,
            &ctx.config.llm,
            &ctx.config.analysis,
            &symbols,
        )?;
        ctx.audit.record("semantic_search", &query, "ok");
        Ok(out)
    }
}

pub struct ExpandContext;
impl Tool for ExpandContext {
    fn name(&self) -> &str {
        "expand_context"
    }
    fn description(&self) -> &str {
        "Контекст вокруг hit: enclosing symbol, imports, соседние строки"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "line": {"type": "integer"},
                "pad": {"type": "integer"}
            },
            "required": ["path", "line"]
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let path = arg_str(args, "path")?;
        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let pad = args.get("pad").and_then(|v| v.as_u64()).unwrap_or(12) as usize;
        let symbols = load_symbols(ctx)?;
        let out = context::expand_context(&ctx.profile.root, &symbols, &path, line, pad)?;
        ctx.audit.record("expand_context", &path, "ok");
        Ok(truncate(&out, ctx.config.run.max_context_chars.min(12_000)))
    }
}

pub struct IndexStatusTool;
impl Tool for IndexStatusTool {
    fn name(&self) -> &str {
        "index_status"
    }
    fn description(&self) -> &str {
        "Здоровье индекса символов: count, stale, last HEAD"
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, _args: &Value) -> Result<String> {
        let status = index::index_status(&ctx.profile)?;
        ctx.audit.record("index_status", ".", "ok");
        Ok(serde_json::to_string_pretty(&status)?)
    }
}

pub struct ChangedSymbols;
impl Tool for ChangedSymbols {
    fn name(&self) -> &str {
        "changed_symbols"
    }
    fn description(&self) -> &str {
        "Символы, затронутые git diff (vs base, по умолчанию HEAD)"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "base": {"type": "string", "description": "git ref, напр. main или HEAD~3"},
                "impact": {"type": "boolean", "description": "добавить callers изменённых"}
            }
        })
    }
    fn kind(&self) -> ToolKind {
        ToolKind::ReadOnly
    }
    fn execute(&self, ctx: &AgentContext, args: &Value) -> Result<String> {
        let base = arg_str_opt(args, "base");
        let want_impact = args
            .get("impact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let symbols = load_symbols(ctx)?;
        let hits = if want_impact {
            let g = graph::ensure_graph(&ctx.profile.root, &symbols, ctx.config.analysis.graph)?;
            changed::impact(&ctx.profile.root, &symbols, &g, base.as_deref())?
        } else {
            changed::changed_symbols(&ctx.profile.root, &symbols, base.as_deref())?
        };
        let (shown, trunc) = truncate_hits(&hits, 60);
        ctx.audit
            .record("changed_symbols", base.as_deref().unwrap_or("HEAD"), "ok");
        Ok(format_hits_json(&shown, trunc))
    }
}
