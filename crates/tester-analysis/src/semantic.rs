use super::{AnalysisHit, format_hits_json, truncate_hits};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tester_core::config::{AnalysisConfig, LlmConfig};
use tester_core::model::Symbol;
use tester_core::security::skip_dir_name;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingChunk {
    id: String,
    path: PathBuf,
    start_line: usize,
    end_line: usize,
    text: String,
    #[serde(default)]
    symbol: Option<String>,
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingStore {
    model: String,
    chunks: Vec<EmbeddingChunk>,
}

fn workspace_hash(root: &Path) -> String {
    let mut h = DefaultHasher::new();
    root.to_string_lossy().hash(&mut h);
    format!("{:x}", h.finish())
}

fn store_path(root: &Path) -> PathBuf {
    root.join(".tester")
        .join("workspace")
        .join(workspace_hash(root))
        .join("embeddings")
        .join("index.json")
}

fn should_skip_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.ends_with(".lock")
        || name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name == "Cargo.lock"
        || name == "package-lock.json"
        || name == "pnpm-lock.yaml"
        || name == "yarn.lock"
    {
        return true;
    }
    path.components()
        .any(|c| c.as_os_str().to_str().map(skip_dir_name).unwrap_or(false))
}

fn chunk_symbols(symbols: &[Symbol]) -> Vec<(PathBuf, usize, usize, String, Option<String>)> {
    let mut out = Vec::new();
    for sym in symbols {
        if sym.body.is_empty() && sym.signature.is_empty() {
            continue;
        }
        let text = format!("{}\n{}", sym.signature, sym.body);
        let name = if sym.qualname.is_empty() {
            sym.name.clone()
        } else {
            sym.qualname.clone()
        };
        out.push((
            sym.file.clone(),
            sym.start_line,
            sym.end_line,
            text,
            Some(name),
        ));
    }
    out
}

fn chunk_files_fallback(
    root: &Path,
    already: &[(PathBuf, usize, usize, String, Option<String>)],
) -> Vec<(PathBuf, usize, usize, String, Option<String>)> {
    let covered: std::collections::HashSet<String> = already
        .iter()
        .map(|(p, _, _, _, _)| p.to_string_lossy().replace('\\', "/"))
        .collect();
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() || should_skip_file(entry.path()) {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !matches!(
            ext,
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "kt"
                | "java"
                | "php"
                | "dart"
                | "cpp"
                | "h"
        ) {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_path_buf();
        let key = rel.to_string_lossy().replace('\\', "/");
        if covered.contains(&key) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let end = (i + 40).min(lines.len());
            let chunk = lines[i..end].join("\n");
            if chunk.trim().len() > 40 {
                out.push((rel.clone(), i + 1, end, chunk, None));
            }
            i += 35;
        }
        if out.len() > 500 {
            break;
        }
    }
    out
}

fn embed_texts(llm: &LlmConfig, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let http = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(llm.timeout_secs.max(30)))
        .build()?;
    let url = format!("{}/embeddings", llm.base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "input": texts,
    });
    let mut req = http.post(&url).json(&body);
    if let Some(key) = &llm.api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send()?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        bail!("embeddings HTTP {status}: {text}");
    }
    let v: serde_json::Value = resp.json()?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| anyhow::anyhow!("embeddings: нет data"))?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let emb = item
            .get("embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| anyhow::anyhow!("embeddings: нет embedding"))?
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
        out.push(emb);
    }
    Ok(out)
}

pub fn build_embeddings(
    root: &Path,
    symbols: &[Symbol],
    llm: &LlmConfig,
    analysis: &AnalysisConfig,
) -> Result<usize> {
    if !analysis.embeddings {
        bail!("embeddings выключены ([analysis] embeddings = true)");
    }
    let model = if analysis.embeddings_model.is_empty() {
        if llm.model.is_empty() {
            bail!("нужен embeddings_model или llm.model");
        }
        llm.model.clone()
    } else {
        analysis.embeddings_model.clone()
    };

    let mut chunks_raw = chunk_symbols(symbols);
    chunks_raw.extend(chunk_files_fallback(root, &chunks_raw));
    chunks_raw.truncate(400);

    let mut store = EmbeddingStore {
        model: model.clone(),
        chunks: Vec::new(),
    };

    const BATCH: usize = 16;
    for batch in chunks_raw.chunks(BATCH) {
        let texts: Vec<String> = batch
            .iter()
            .map(|(_, _, _, t, _)| t.chars().take(4000).collect())
            .collect();
        let embeds = embed_texts(llm, &model, &texts)?;
        for (i, (path, start, end, text, sym)) in batch.iter().enumerate() {
            let emb = embeds.get(i).cloned().unwrap_or_default();
            store.chunks.push(EmbeddingChunk {
                id: format!("{}:{}:{}", path.display(), start, end),
                path: path.clone(),
                start_line: *start,
                end_line: *end,
                text: text.chars().take(500).collect(),
                symbol: sym.clone(),
                embedding: emb,
            });
        }
    }

    let path = store_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string(&store)?)?;
    Ok(store.chunks.len())
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..a.len() {
        let x = a[i] as f64;
        let y = b[i] as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

pub fn semantic_search(
    root: &Path,
    query: &str,
    llm: &LlmConfig,
    analysis: &AnalysisConfig,
    symbols: &[Symbol],
) -> Result<String> {
    if !analysis.embeddings {
        bail!("semantic_search: включите [analysis] embeddings = true и постройте индекс");
    }
    let path = store_path(root);
    if !path.exists() {
        let _ = build_embeddings(root, symbols, llm, analysis)?;
    }
    let text = std::fs::read_to_string(&path)?;
    let store: EmbeddingStore = serde_json::from_str(&text)?;
    if store.chunks.is_empty() {
        return Ok(r#"{"hits":[],"count":0}"#.into());
    }

    let model = if analysis.embeddings_model.is_empty() {
        store.model.clone()
    } else {
        analysis.embeddings_model.clone()
    };
    let q_emb = embed_texts(llm, &model, &[query.to_string()])?
        .into_iter()
        .next()
        .unwrap_or_default();

    let top_k = analysis.semantic_top_k.max(1) as usize;
    let q_l = query.to_lowercase();
    let mut scored: Vec<(f64, usize)> = store
        .chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut score = cosine(&q_emb, &c.embedding);
            if let Some(sym) = &c.symbol {
                if q_l.contains(&sym.to_lowercase()) {
                    score += 0.15;
                }
            }
            let path_s = c.path.to_string_lossy().to_lowercase();
            for tok in q_l.split_whitespace() {
                if tok.len() > 3 && path_s.contains(tok) {
                    score += 0.05;
                }
            }
            (score, i)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let hits: Vec<AnalysisHit> = scored
        .into_iter()
        .take(top_k)
        .filter_map(|(score, i)| {
            let c = store.chunks.get(i)?;
            Some(AnalysisHit {
                path: c.path.clone(),
                start_line: c.start_line,
                end_line: c.end_line,
                symbol: c.symbol.clone(),
                kind: Some("chunk".into()),
                signature: None,
                score,
                why: "semantic".into(),
                snippet: Some(c.text.chars().take(200).collect()),
            })
        })
        .collect();
    let (shown, trunc) = truncate_hits(&hits, top_k);
    Ok(format_hits_json(&shown, trunc))
}
