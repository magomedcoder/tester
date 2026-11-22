use super::AnalysisHit;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use tester_core::model::Symbol;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeGraph {
    pub nodes: BTreeMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String, // call | import | type | test
}

fn graph_path(root: &Path) -> PathBuf {
    root.join(".tester").join("graph").join("graph.json")
}

fn load_graph(root: &Path) -> Result<Option<CodeGraph>> {
    let path = graph_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

fn save_graph(root: &Path, graph: &CodeGraph) -> Result<()> {
    let path = graph_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(graph)?)?;
    Ok(())
}

fn build_graph(root: &Path, symbols: &[Symbol]) -> Result<CodeGraph> {
    let mut graph = CodeGraph::default();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();

    for sym in symbols {
        let id = node_id(sym);
        graph.nodes.insert(
            id.clone(),
            GraphNode {
                id: id.clone(),
                name: sym.name.clone(),
                kind: sym.kind.clone(),
                file: sym.file.clone(),
                start_line: sym.start_line,
                end_line: sym.end_line,
            },
        );
        by_name.entry(sym.name.clone()).or_default().push(id);
    }

    for sym in symbols {
        let from = node_id(sym);
        let body = &sym.body;
        if body.is_empty() {
            continue;
        }
        for (name, ids) in &by_name {
            if name.len() < 3 || name == &sym.name {
                continue;
            }
            if body.contains(name.as_str()) {
                for to in ids {
                    if to == &from {
                        continue;
                    }
                    graph.edges.push(GraphEdge {
                        from: from.clone(),
                        to: to.clone(),
                        kind: "call".into(),
                    });
                }
            }
        }
    }

    collect_import_edges(root, &mut graph)?;

    let mut seen = BTreeSet::new();
    graph
        .edges
        .retain(|e| seen.insert((e.from.clone(), e.to.clone(), e.kind.clone())));

    save_graph(root, &graph)?;
    Ok(graph)
}

fn node_id(sym: &Symbol) -> String {
    let q = if sym.qualname.is_empty() {
        &sym.name
    } else {
        &sym.qualname
    };
    format!("{}:{}:{}", sym.file.display(), sym.start_line, q)
}

fn collect_import_edges(root: &Path, graph: &mut CodeGraph) -> Result<()> {
    use tester_core::security::skip_dir_name;
    use walkdir::WalkDir;

    let import_re = regex::Regex::new(
        r#"(?m)^(?:use\s+([\w:]+)|import\s+.*?from\s+['"]([^'"]+)|from\s+([\w.]+)\s+import|require\(['"]([^'"]+)['"]\))"#,
    )?;

    let mut file_nodes: HashMap<String, Vec<String>> = HashMap::new();
    for (id, node) in &graph.nodes {
        let key = node.file.to_string_lossy().replace('\\', "/");
        file_nodes.entry(key).or_default().push(id.clone());
    }

    for entry in WalkDir::new(root)
        .max_depth(12)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str().to_str().map(skip_dir_name).unwrap_or(false))
        {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !matches!(
            ext,
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "kt" | "java" | "php" | "dart"
        ) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        let Some(from_nodes) = file_nodes.get(&rel) else {
            continue;
        };
        for caps in import_re.captures_iter(&text) {
            let target = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .or_else(|| caps.get(4))
                .map(|m| m.as_str())
                .unwrap_or("");
            if target.is_empty() {
                continue;
            }
            let seg = target.rsplit([':', '/', '.']).next().unwrap_or(target);
            for (fid, nodes) in &file_nodes {
                if fid.contains(seg) && fid != &rel {
                    for from in from_nodes {
                        for to in nodes.iter().take(3) {
                            graph.edges.push(GraphEdge {
                                from: from.clone(),
                                to: to.clone(),
                                kind: "import".into(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn ensure_graph(root: &Path, symbols: &[Symbol], enabled: bool) -> Result<CodeGraph> {
    if !enabled {
        return Ok(CodeGraph::default());
    }
    if let Some(g) = load_graph(root)? {
        if !g.nodes.is_empty() {
            return Ok(g);
        }
    }
    build_graph(root, symbols)
}

pub fn lookup(graph: &CodeGraph, query: &str) -> Vec<AnalysisHit> {
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    for node in graph.nodes.values() {
        if node.name.to_lowercase().contains(&q) || node.id.to_lowercase().contains(&q) {
            hits.push(AnalysisHit {
                path: node.file.clone(),
                start_line: node.start_line,
                end_line: node.end_line,
                symbol: Some(node.name.clone()),
                kind: Some(node.kind.clone()),
                signature: None,
                score: if node.name.eq_ignore_ascii_case(query) {
                    100.0
                } else {
                    50.0
                },
                why: "graph-lookup".into(),
                snippet: None,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

fn resolve_ids(graph: &CodeGraph, query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    graph
        .nodes
        .values()
        .filter(|n| n.name.to_lowercase() == q || n.id.to_lowercase().contains(&q))
        .map(|n| n.id.clone())
        .collect()
}

pub fn callers(graph: &CodeGraph, query: &str, depth: usize) -> Vec<AnalysisHit> {
    neighbors_dir(graph, query, depth, true)
}

pub fn callees(graph: &CodeGraph, query: &str, depth: usize) -> Vec<AnalysisHit> {
    neighbors_dir(graph, query, depth, false)
}

fn neighbors_dir(graph: &CodeGraph, query: &str, depth: usize, incoming: bool) -> Vec<AnalysisHit> {
    let starts = resolve_ids(graph, query);
    let mut visited = BTreeSet::new();
    let mut queue: VecDeque<(String, usize)> = starts.into_iter().map(|id| (id, 0)).collect();
    let mut hits = Vec::new();

    while let Some((id, d)) = queue.pop_front() {
        if d >= depth.max(1) {
            continue;
        }
        for edge in &graph.edges {
            if edge.kind != "call" && edge.kind != "import" {
                continue;
            }
            let (next, _via) = if incoming {
                if edge.to == id {
                    (edge.from.clone(), &edge.from)
                } else {
                    continue;
                }
            } else if edge.from == id {
                (edge.to.clone(), &edge.to)
            } else {
                continue;
            };
            if !visited.insert(next.clone()) {
                continue;
            }
            if let Some(node) = graph.nodes.get(&next) {
                hits.push(AnalysisHit {
                    path: node.file.clone(),
                    start_line: node.start_line,
                    end_line: node.end_line,
                    symbol: Some(node.name.clone()),
                    kind: Some(edge.kind.clone()),
                    signature: None,
                    score: (depth - d) as f64,
                    why: if incoming {
                        format!("caller depth={}", d + 1)
                    } else {
                        format!("callee depth={}", d + 1)
                    },
                    snippet: None,
                });
                queue.push_back((next, d + 1));
            }
        }
    }
    hits
}

pub fn neighbors(graph: &CodeGraph, query: &str) -> Vec<AnalysisHit> {
    let mut hits = callers(graph, query, 1);
    hits.extend(callees(graph, query, 1));
    hits
}

pub fn shortest_path(graph: &CodeGraph, from: &str, to: &str, limit: usize) -> Vec<String> {
    let starts = resolve_ids(graph, from);
    let goals: BTreeSet<String> = resolve_ids(graph, to).into_iter().collect();
    if starts.is_empty() || goals.is_empty() {
        return vec![];
    }

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for e in &graph.edges {
        adj.entry(e.from.clone()).or_default().push(e.to.clone());
    }

    for start in starts {
        let mut prev: HashMap<String, String> = HashMap::new();
        let mut q = VecDeque::new();
        q.push_back(start.clone());
        prev.insert(start.clone(), start.clone());
        let mut found = None;
        while let Some(cur) = q.pop_front() {
            if goals.contains(&cur) && cur != start {
                found = Some(cur);
                break;
            }
            if prev.len() > limit.max(32) {
                break;
            }
            for nxt in adj.get(&cur).into_iter().flatten() {
                if prev.contains_key(nxt) {
                    continue;
                }
                prev.insert(nxt.clone(), cur.clone());
                q.push_back(nxt.clone());
            }
        }
        if let Some(end) = found {
            let mut path = vec![end.clone()];
            let mut cur = end;
            while let Some(p) = prev.get(&cur) {
                if p == &cur {
                    break;
                }
                path.push(p.clone());
                cur = p.clone();
            }
            path.reverse();
            return path
                .into_iter()
                .filter_map(|id| graph.nodes.get(&id).map(|n| n.name.clone()))
                .collect();
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_call_edge() {
        let dir = tempfile::tempdir().unwrap();
        let symbols = vec![
            Symbol {
                name: "alpha_fn".into(),
                kind: "fn".into(),
                file: PathBuf::from("src/a.rs"),
                start_line: 1,
                end_line: 3,
                signature: "fn alpha_fn()".into(),
                body: "{ beta_fn(); }".into(),
                is_public: true,
                uses_io: false,
                branch_score: 1,
                qualname: "alpha_fn".into(),
                visibility: "pub".into(),
                parent: None,
                docstring: String::new(),
                start_byte: 0,
            },
            Symbol {
                name: "beta_fn".into(),
                kind: "fn".into(),
                file: PathBuf::from("src/b.rs"),
                start_line: 1,
                end_line: 2,
                signature: "fn beta_fn()".into(),
                body: "{}".into(),
                is_public: true,
                uses_io: false,
                branch_score: 1,
                qualname: "beta_fn".into(),
                visibility: "pub".into(),
                parent: None,
                docstring: String::new(),
                start_byte: 0,
            },
        ];
        let g = build_graph(dir.path(), &symbols).unwrap();
        assert!(g.edges.iter().any(|e| e.kind == "call"));
        let c = callers(&g, "beta_fn", 1);
        assert!(!c.is_empty());
    }
}
