#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
)]

pub mod changed;
pub mod context;
#[cfg(test)]
mod eval;
pub mod graph;
pub mod map;
pub mod search;
pub mod semantic;

use serde::Serialize;
use std::path::PathBuf;
use tester_core::model::Symbol;

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisHit {
    pub path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub score: f64,
    pub why: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

impl AnalysisHit {
    pub fn from_symbol(sym: &Symbol, score: f64, why: impl Into<String>) -> Self {
        Self {
            path: sym.file.clone(),
            start_line: sym.start_line,
            end_line: sym.end_line,
            symbol: Some(if sym.qualname.is_empty() {
                sym.name.clone()
            } else {
                sym.qualname.clone()
            }),
            kind: Some(sym.kind.clone()),
            signature: Some(sym.signature.clone()),
            score,
            why: why.into(),
            snippet: None,
        }
    }

    pub fn with_snippet(mut self, snip: impl Into<String>) -> Self {
        self.snippet = Some(snip.into());
        self
    }
}

pub fn truncate_hits(hits: &[AnalysisHit], max: usize) -> (Vec<AnalysisHit>, usize) {
    if hits.len() <= max {
        return (hits.to_vec(), 0);
    }
    (hits[..max].to_vec(), hits.len() - max)
}

pub fn format_hits_json(hits: &[AnalysisHit], truncated: usize) -> String {
    let mut v = serde_json::json!({
        "hits": hits,
        "count": hits.len(),
    });
    if truncated > 0 {
        v["truncated"] = serde_json::json!(truncated);
        v["note"] = serde_json::json!(format!("and {truncated} more"));
    }
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "[]".into())
}
