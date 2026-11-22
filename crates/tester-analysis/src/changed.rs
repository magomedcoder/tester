use super::AnalysisHit;
use crate::graph::{self, CodeGraph};
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use tester_core::model::Symbol;

pub fn changed_symbols(
    root: &Path,
    symbols: &[Symbol],
    base: Option<&str>,
) -> Result<Vec<AnalysisHit>> {
    let ranges = diff_line_ranges(root, base)?;
    let mut hits = Vec::new();
    for sym in symbols {
        let file = sym.file.to_string_lossy().replace('\\', "/");
        let Some(lines) = ranges.get(&file) else {
            continue;
        };
        if lines
            .iter()
            .any(|&l| l >= sym.start_line && l <= sym.end_line)
        {
            hits.push(AnalysisHit::from_symbol(sym, 1.0, "changed-in-diff"));
        }
    }
    Ok(hits)
}

fn diff_line_ranges(
    root: &Path,
    base: Option<&str>,
) -> Result<std::collections::HashMap<String, BTreeSet<usize>>> {
    let base_ref = base.unwrap_or("HEAD");
    let args = if base_ref == "HEAD" {
        vec!["diff".into(), "-U0".into(), "HEAD".into()]
    } else {
        vec!["diff".into(), "-U0".into(), format!("{base_ref}...HEAD")]
    };
    let out = Command::new("git").args(&args).current_dir(root).output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    parse_diff_ranges(&text)
}

fn parse_diff_ranges(diff: &str) -> Result<std::collections::HashMap<String, BTreeSet<usize>>> {
    let mut map = std::collections::HashMap::new();
    let mut current: Option<String> = None;
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            current = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let Some(file) = current.clone() else {
                continue;
            };
            if let Some(plus) = rest.split_whitespace().find(|t| t.starts_with('+')) {
                let nums = plus.trim_start_matches('+');
                let (start, count) = if let Some((a, b)) = nums.split_once(',') {
                    (
                        a.parse::<usize>().unwrap_or(1),
                        b.parse::<usize>().unwrap_or(1),
                    )
                } else {
                    (nums.parse::<usize>().unwrap_or(1), 1)
                };
                let set = map.entry(file).or_insert_with(BTreeSet::new);
                for l in start..start.saturating_add(count.max(1)) {
                    set.insert(l);
                }
            }
        }
    }
    Ok(map)
}

pub fn impact(
    root: &Path,
    symbols: &[Symbol],
    graph: &CodeGraph,
    base: Option<&str>,
) -> Result<Vec<AnalysisHit>> {
    let changed = changed_symbols(root, symbols, base)?;
    let mut hits = changed.clone();
    let mut seen: BTreeSet<String> = hits.iter().filter_map(|h| h.symbol.clone()).collect();
    for h in &changed {
        let Some(name) = &h.symbol else {
            continue;
        };
        for c in graph::callers(graph, name, 2) {
            let key = c.symbol.clone().unwrap_or_default();
            if seen.insert(key) {
                let mut hit = c;
                hit.why = format!("impact:{}", hit.why);
                hits.push(hit);
            }
        }
    }
    Ok(hits)
}

pub fn stacktrace_to_symbols(symbols: &[Symbol], stack: &str) -> Vec<AnalysisHit> {
    let mut hits = Vec::new();
    for line in stack.lines() {
        let trimmed = line.trim();
        let mut file_hint: Option<&str> = None;
        let mut line_no: Option<usize> = None;
        for token in trimmed.split_whitespace() {
            if let Some((f, l)) = token.rsplit_once(':') {
                if let Ok(n) = l.parse::<usize>() {
                    if f.contains('.') || f.contains('/') {
                        file_hint = Some(f.trim_matches(|c| c == '(' || c == ')'));
                        line_no = Some(n);
                    }
                }
            }
        }
        if let (Some(file), Some(ln)) = (file_hint, line_no) {
            for sym in symbols {
                let f = sym.file.to_string_lossy();
                if f.ends_with(file) || file.ends_with(f.as_ref()) {
                    if sym.start_line <= ln && sym.end_line >= ln {
                        hits.push(AnalysisHit::from_symbol(
                            sym,
                            10.0,
                            format!("stack:{file}:{ln}"),
                        ));
                    }
                }
            }
        }
        for sym in symbols {
            if trimmed.contains(&sym.name) && sym.name.len() > 2 {
                hits.push(AnalysisHit::from_symbol(sym, 3.0, "stack-name"));
            }
        }
    }
    let mut seen = BTreeSet::new();
    hits.retain(|h| {
        seen.insert((
            h.path.to_string_lossy().to_string(),
            h.start_line,
            h.symbol.clone().unwrap_or_default(),
        ))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unified_diff() {
        let diff = r#"
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -10,0 +11,2 @@
+fn new() {}
+fn other() {}
"#;
        let map = parse_diff_ranges(diff).unwrap();
        let lines = map.get("src/a.rs").unwrap();
        assert!(lines.contains(&11));
        assert!(lines.contains(&12));
    }
}
