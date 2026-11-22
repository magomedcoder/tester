use super::graph;
use super::search::{self, SymbolQuery};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tester_lang::discover::discover;
use tester_lang::index::index_symbols;

#[derive(Debug, Serialize)]
pub struct AnalysisEvalCase {
    pub query: String,
    pub expected_path_substr: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct AnalysisEvalReport {
    pub recall_at_5: f64,
    pub mrr: f64,
    pub cases: usize,
    pub hits: usize,
}

pub fn default_cases() -> Vec<AnalysisEvalCase> {
    vec![
        AnalysisEvalCase {
            query: "add".into(),
            expected_path_substr: "lib.rs".into(),
            kind: "exact".into(),
        },
        AnalysisEvalCase {
            query: "multiply".into(),
            expected_path_substr: "lib.rs".into(),
            kind: "exact".into(),
        },
    ]
}

pub fn eval_analysis(root: &Path, cases: &[AnalysisEvalCase]) -> Result<AnalysisEvalReport> {
    let profile = discover(root)?;
    let symbols = index_symbols(&profile)?;
    let g = graph::ensure_graph(root, &symbols, true)?;

    let mut hits = 0usize;
    let mut rr_sum = 0.0f64;
    for case in cases {
        let found = match case.kind.as_str() {
            "callers" => {
                let c = graph::callers(&g, &case.query, 1);
                c.iter().position(|h| {
                    h.path
                        .to_string_lossy()
                        .contains(&case.expected_path_substr)
                })
            }
            _ => {
                let q = SymbolQuery {
                    name: case.query.clone(),
                    max: 5,
                    ..Default::default()
                };
                let results = search::find_symbol(&symbols, &q);
                results.iter().position(|h| {
                    h.path
                        .to_string_lossy()
                        .contains(&case.expected_path_substr)
                })
            }
        };
        if let Some(rank) = found {
            if rank < 5 {
                hits += 1;
            }
            rr_sum += 1.0 / (rank as f64 + 1.0);
        }
    }
    let n = cases.len().max(1) as f64;
    Ok(AnalysisEvalReport {
        recall_at_5: hits as f64 / n,
        mrr: rr_sum / n,
        cases: cases.len(),
        hits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_on_fixture() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"e\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn add(a:i32,b:i32)->i32{a+b}\npub fn multiply(a:i32,b:i32)->i32{a*b}\n",
        )
        .unwrap();
        let report = eval_analysis(dir.path(), &default_cases()).unwrap();
        assert!(report.recall_at_5 >= 0.5, "{report:?}");
    }
}
