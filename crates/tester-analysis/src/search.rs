use super::{AnalysisHit, format_hits_json, truncate_hits};
use std::path::Path;
use tester_core::model::Symbol;

#[derive(Debug, Clone, Default)]
pub struct SymbolQuery {
    pub name: String,
    pub kind: Option<String>,
    pub file: Option<String>,
    pub module: Option<String>,
    pub visibility: Option<String>,
    pub include_tests: bool,
    pub max: usize,
}

pub fn find_symbol(symbols: &[Symbol], query: &SymbolQuery) -> Vec<AnalysisHit> {
    let needle = query.name.trim();
    if needle.is_empty() {
        return vec![];
    }
    let needle_l = needle.to_lowercase();
    let max = query.max.max(1);

    let mut scored: Vec<(f64, String, &Symbol)> = Vec::new();
    for sym in symbols {
        if !query.include_tests && sym.kind == "test" {
            continue;
        }
        if let Some(k) = &query.kind {
            if !sym.kind.eq_ignore_ascii_case(k) {
                continue;
            }
        }
        if let Some(f) = &query.file {
            let file = sym.file.to_string_lossy();
            if !file.contains(f) {
                continue;
            }
        }
        if let Some(m) = &query.module {
            let q = if sym.qualname.is_empty() {
                &sym.name
            } else {
                &sym.qualname
            };
            if !q.contains(m) && !sym.file.to_string_lossy().contains(m.as_str()) {
                continue;
            }
        }
        if let Some(v) = &query.visibility {
            let vis = if sym.visibility.is_empty() {
                if sym.is_public { "pub" } else { "private" }
            } else {
                sym.visibility.as_str()
            };
            if !vis.eq_ignore_ascii_case(v) {
                continue;
            }
        }

        let name_l = sym.name.to_lowercase();
        let qual_l = sym.qualname.to_lowercase();
        let (score, why) = rank_match(needle, &needle_l, &sym.name, &name_l, &qual_l, sym);
        if score > 0.0 {
            scored.push((score, why, sym));
        }
    }

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.file.cmp(&b.2.file))
            .then_with(|| a.2.start_line.cmp(&b.2.start_line))
    });
    scored.truncate(max);
    scored
        .into_iter()
        .map(|(score, why, sym)| AnalysisHit::from_symbol(sym, score, why))
        .collect()
}

fn rank_match(
    needle: &str,
    needle_l: &str,
    name: &str,
    name_l: &str,
    qual_l: &str,
    sym: &Symbol,
) -> (f64, String) {
    let mut score = 0.0;
    let mut why = String::new();

    if name == needle || qual_l == needle_l {
        score += 100.0;
        why.push_str("exact");
    } else if name_l == needle_l {
        score += 90.0;
        why.push_str("exact-ci");
    } else if name_l.starts_with(needle_l) || qual_l.starts_with(needle_l) {
        score += 70.0;
        why.push_str("prefix");
    } else if name_l.contains(needle_l) || qual_l.contains(needle_l) {
        score += 50.0;
        why.push_str("contains");
    } else if fuzzy_score(needle_l, name_l) > 0.55 {
        score += 30.0 * fuzzy_score(needle_l, name_l);
        why.push_str("fuzzy");
    } else {
        return (0.0, String::new());
    }

    if sym.is_public {
        score += 8.0;
        why.push_str(";pub");
    } else {
        score -= 2.0;
        why.push_str(";private");
    }
    if sym.kind == "test" {
        score -= 15.0;
        why.push_str(";test");
    } else {
        why.push_str(";prod");
    }
    let path = sym.file.to_string_lossy();
    if path.contains("src/") {
        score += 3.0;
    }
    if path.contains("/generated/") || path.contains(".min.") {
        score -= 20.0;
    }

    (score, why)
}

fn fuzzy_score(needle: &str, hay: &str) -> f64 {
    if needle.is_empty() || hay.is_empty() {
        return 0.0;
    }
    let nb: Vec<char> = needle.chars().collect();
    let hb: Vec<char> = hay.chars().collect();
    let mut i = 0;
    for &c in &hb {
        if i < nb.len() && c == nb[i] {
            i += 1;
        }
    }
    if i < nb.len() {
        return 0.0;
    }
    needle.len() as f64 / hay.len().max(1) as f64
}

pub fn find_definition(symbols: &[Symbol], name: &str) -> Option<AnalysisHit> {
    let q = SymbolQuery {
        name: name.into(),
        max: 1,
        ..Default::default()
    };
    find_symbol(symbols, &q).into_iter().next()
}

pub fn find_references_text(
    root: &Path,
    symbols: &[Symbol],
    name: &str,
    max: usize,
) -> Vec<AnalysisHit> {
    use tester_core::security::skip_dir_name;
    use walkdir::WalkDir;

    let def = find_definition(symbols, name);
    let needle = def
        .as_ref()
        .and_then(|h| h.symbol.clone())
        .unwrap_or_else(|| name.to_string());
    let short = needle.rsplit("::").next().unwrap_or(&needle).to_string();

    let mut hits = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
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
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_path_buf();
        for (i, line) in text.lines().enumerate() {
            if line.contains(&short) {
                if let Some(d) = &def {
                    if d.path == rel && d.start_line == i + 1 {
                        continue;
                    }
                }
                hits.push(AnalysisHit {
                    path: rel.clone(),
                    start_line: i + 1,
                    end_line: i + 1,
                    symbol: Some(short.clone()),
                    kind: Some("ref".into()),
                    signature: None,
                    score: 1.0,
                    why: "text-match".into(),
                    snippet: Some(line.trim().to_string()),
                });
                if hits.len() >= max {
                    return hits;
                }
            }
        }
    }
    hits
}

pub fn outline_file(symbols: &[Symbol], file: &Path) -> Vec<AnalysisHit> {
    let file_s = file.to_string_lossy().replace('\\', "/");
    let mut out: Vec<_> = symbols
        .iter()
        .filter(|s| {
            let f = s.file.to_string_lossy().replace('\\', "/");
            f == file_s || f.ends_with(&file_s) || file_s.ends_with(&f)
        })
        .map(|s| AnalysisHit::from_symbol(s, 1.0, "outline"))
        .collect();
    out.sort_by_key(|h| h.start_line);
    out
}

pub fn render_symbol_search(symbols: &[Symbol], query: &SymbolQuery) -> String {
    let hits = find_symbol(symbols, query);
    let (shown, trunc) = truncate_hits(&hits, query.max.max(1));
    format_hits_json(&shown, trunc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sym(name: &str, file: &str, public: bool, kind: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind: kind.into(),
            file: PathBuf::from(file),
            start_line: 1,
            end_line: 5,
            signature: format!("fn {name}()"),
            body: String::new(),
            is_public: public,
            uses_io: false,
            branch_score: 1,
            qualname: name.into(),
            visibility: if public {
                "pub".into()
            } else {
                "private".into()
            },
            parent: None,
            docstring: String::new(),
            start_byte: 0,
        }
    }

    #[test]
    fn ranks_exact_over_fuzzy() {
        let symbols = vec![
            sym("add_extra", "src/a.rs", true, "fn"),
            sym("add", "src/b.rs", true, "fn"),
            sym("add", "src/c.rs", false, "test"),
        ];
        let hits = find_symbol(
            &symbols,
            &SymbolQuery {
                name: "add".into(),
                max: 5,
                ..Default::default()
            },
        );
        assert_eq!(hits[0].symbol.as_deref(), Some("add"));
        assert!(hits[0].path.to_string_lossy().contains("b.rs"));
    }
}
