use crate::index::iter_source_files;
use anyhow::Result;
use regex::Regex;
use std::path::Path;
use tester_core::model::Symbol;

pub(crate) fn index_typescript(root: &Path) -> Result<Vec<Symbol>> {
    let export_fn = Regex::new(
        r"(?m)^export\s+(?:async\s+)?function\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^)]*\))",
    )?;
    let export_const = Regex::new(
        r"(?m)^export\s+const\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\(",
    )?;
    let export_class = Regex::new(
        r"(?m)^export\s+(?:default\s+)?(?:abstract\s+)?class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    )?;
    let export_interface =
        Regex::new(r"(?m)^export\s+(?:type|interface)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")?;
    let method_re = Regex::new(
        r"(?m)^(?P<indent>\s+)(?:public\s+|private\s+|protected\s+|async\s+|static\s+)*(?:async\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^)]*\))\s*\{",
    )?;

    let mut out = Vec::new();
    for file in iter_source_files(root, &["ts", "tsx", "js", "jsx"]) {
        let rel = file.strip_prefix(root).unwrap_or(&file);
        let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let is_test = name.contains(".test.") || name.contains(".spec.");
        if name.ends_with(".d.ts") {
            continue;
        }
        if rel.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "node_modules"
        }) {
            continue;
        }
        if is_generated_or_minified(&file, name) {
            continue;
        }

        let text = std::fs::read_to_string(&file)?;
        let module = module_from_path(rel);

        for caps in export_class.captures_iter(&text) {
            let n = caps.name("name").unwrap().as_str();
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            let is_default = caps.get(0).unwrap().as_str().contains("default");
            out.push(
                Symbol {
                    name: n.into(),
                    kind: if is_test {
                        "test".into()
                    } else {
                        "class".into()
                    },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line,
                    signature: caps.get(0).unwrap().as_str().trim().to_string(),
                    body: extract_brace_or_arrow(&text, caps.get(0).unwrap().end()),
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}.{n}"),
                    visibility: "export".into(),
                    parent: None,
                    docstring: leading_comment(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
            let _ = is_default;
        }

        for caps in export_interface.captures_iter(&text) {
            let n = caps.name("name").unwrap().as_str();
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            let kind = if caps.get(0).unwrap().as_str().contains("interface") {
                "interface"
            } else {
                "type"
            };
            out.push(
                Symbol {
                    name: n.into(),
                    kind: kind.into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line,
                    signature: caps.get(0).unwrap().as_str().trim().to_string(),
                    body: String::new(),
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}.{n}"),
                    visibility: "export".into(),
                    parent: None,
                    docstring: leading_comment(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
        }

        for caps in export_fn.captures_iter(&text) {
            let mut sym = symbol_from_match(&text, rel, &caps, "function", &module);
            if is_test {
                sym.kind = "test".into();
            }
            out.push(sym.finalize());
        }
        for caps in export_const.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            let body = extract_brace_or_arrow(&text, caps.get(0).unwrap().end());
            out.push(
                Symbol {
                    name: name.into(),
                    kind: if is_test {
                        "test".into()
                    } else {
                        "const-fn".into()
                    },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("export const {name}"),
                    body,
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}.{name}"),
                    visibility: "export".into(),
                    parent: None,
                    docstring: leading_comment(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
        }

        let class_names: Vec<(usize, usize, String)> = export_class
            .captures_iter(&text)
            .filter_map(|caps| {
                let n = caps.name("name")?.as_str().to_string();
                let start = caps.get(0)?.start();
                let body = extract_brace_or_arrow(&text, caps.get(0)?.end());
                Some((start, start + body.len(), n))
            })
            .collect();
        for caps in method_re.captures_iter(&text) {
            let n = caps.name("name").unwrap().as_str();
            if matches!(
                n,
                "if" | "for" | "while" | "switch" | "catch" | "constructor"
            ) {
                if n != "constructor" {
                    continue;
                }
            }
            let start = caps.get(0).unwrap().start();
            let parent = class_names
                .iter()
                .find(|(a, b, _)| start >= *a && start <= *b)
                .map(|(_, _, c)| c.clone());
            let Some(parent) = parent else {
                continue;
            };
            let line = byte_line(&text, start);
            let sig = caps.name("sig").map(|m| m.as_str()).unwrap_or("()");
            let body = extract_brace_or_arrow(&text, caps.get(0).unwrap().end());
            out.push(
                Symbol {
                    name: n.into(),
                    kind: if is_test {
                        "test".into()
                    } else {
                        "method".into()
                    },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line + body.lines().count().max(1) - 1,
                    signature: format!("{parent}.{n}{sig}"),
                    body,
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}.{parent}.{n}"),
                    visibility: "export".into(),
                    parent: Some(parent),
                    docstring: String::new(),
                    start_byte: start,
                }
                .finalize(),
            );
        }
    }
    Ok(out)
}

fn is_generated_or_minified(path: &Path, name: &str) -> bool {
    name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || path.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("generated" | "vendor" | ".next" | "dist" | "build")
            )
        })
}

fn module_from_path(rel: &Path) -> String {
    let s = rel
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
        .replace('/', ".");
    s.trim_start_matches("src.").to_string()
}

fn symbol_from_match(
    text: &str,
    rel: &Path,
    caps: &regex::Captures,
    kind: &str,
    module: &str,
) -> Symbol {
    let name = caps.name("name").unwrap().as_str();
    let sig = caps.name("sig").map(|m| m.as_str()).unwrap_or("()");
    let start = caps.get(0).unwrap().start();
    let line = byte_line(text, start);
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
        qualname: format!("{module}.{name}"),
        visibility: "export".into(),
        parent: None,
        docstring: leading_comment(text, start),
        start_byte: start,
    }
}

fn leading_comment(text: &str, byte_idx: usize) -> String {
    let before = &text[..byte_idx];
    let mut docs = Vec::new();
    for line in before.lines().rev().take(6) {
        let t = line.trim();
        if t.starts_with("/**") || t.starts_with('*') || t.starts_with("//") {
            docs.push(
                t.trim_start_matches(['/', '*'])
                    .trim_end_matches(['/', '*'])
                    .trim()
                    .to_string(),
            );
        } else if t.is_empty() {
            continue;
        } else {
            break;
        }
    }
    docs.reverse();
    docs.into_iter()
        .filter(|s| !s.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

fn byte_line(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].bytes().filter(|b| *b == b'\n').count() + 1
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
