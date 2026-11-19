use crate::index::iter_source_files;
use crate::model::Symbol;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

pub fn index_typescript(root: &Path) -> Result<Vec<Symbol>> {
    let export_fn = Regex::new(
        r"(?m)^export\s+(?:async\s+)?function\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^)]*\))",
    )?;
    let export_const = Regex::new(
        r"(?m)^export\s+const\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\(",
    )?;

    let mut out = Vec::new();
    for file in iter_source_files(root, &["ts", "tsx", "js", "jsx"]) {
        let rel = file.strip_prefix(root).unwrap_or(&file);
        let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.contains(".test.") || name.contains(".spec.") || name.ends_with(".d.ts") {
            continue;
        }
        if rel.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "e2e" || s == "node_modules"
        }) {
            continue;
        }

        let text = std::fs::read_to_string(&file)?;
        for caps in export_fn.captures_iter(&text) {
            out.push(symbol_from_match(&text, rel, &caps, "function"));
        }
        for caps in export_const.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let start = caps.get(0).unwrap().start();
            let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
            let body = extract_brace_or_arrow(&text, caps.get(0).unwrap().end());
            out.push(Symbol {
                name: name.into(),
                kind: "const-fn".into(),
                file: rel.to_path_buf(),
                start_line: line,
                end_line: line + body.lines().count().max(1) - 1,
                signature: format!("export const {name}"),
                body,
                is_public: true,
                uses_io: false,
                branch_score: 1,
            });
        }
    }
    Ok(out)
}

fn symbol_from_match(text: &str, rel: &Path, caps: &regex::Captures, kind: &str) -> Symbol {
    let name = caps.name("name").unwrap().as_str();
    let sig = caps.name("sig").map(|m| m.as_str()).unwrap_or("()");
    let start = caps.get(0).unwrap().start();
    let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    let body = extract_brace_or_arrow(text, caps.get(0).unwrap().end());
    let uses_io = body.contains("fetch(")
        || body.contains("http.")
        || body.contains("fs.")
        || body.contains("express");
    let branch_score = [" if ", "\nif ", " ? ", " switch "]
        .iter()
        .map(|k| body.matches(k).count() as u32)
        .sum::<u32>()
        .max(1);

    Symbol {
        name: name.into(),
        kind: kind.into(),
        file: rel.to_path_buf(),
        start_line: line,
        end_line: line + body.lines().count().max(1) - 1,
        signature: format!("export function {name}{sig}"),
        body,
        is_public: true,
        uses_io,
        branch_score,
    }
}

fn extract_brace_or_arrow(text: &str, from: usize) -> String {
    let rest = &text[from..];
    if let Some(i) = rest.find('{') {
        let mut depth = 0usize;
        let bytes = rest.as_bytes();
        for j in i..bytes.len() {
            match bytes[j] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return rest[i..=j].to_string();
                    }
                }
                _ => {}
            }
        }
    }
    rest.lines().next().unwrap_or("").to_string()
}
