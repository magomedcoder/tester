use crate::index::iter_source_files;
use anyhow::Result;
use regex::Regex;
use std::path::Path;
use tester_core::model::Symbol;

pub(crate) fn index_rust(root: &Path) -> Result<Vec<Symbol>> {
    let fn_re = Regex::new(
        r"(?m)^(?P<indent>\s*)(?P<vis>pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^;{]*)",
    )?;
    let struct_re = Regex::new(
        r"(?m)^(?P<vis>pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|type)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    )?;
    let const_re = Regex::new(
        r"(?m)^(?P<vis>pub(?:\([^)]*\))?\s+)?(?:const|static)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    )?;
    let impl_re = Regex::new(
        r"(?m)^(?:pub(?:\([^)]*\))?\s+)?impl(?:\s*<[^>]+>)?\s+(?:(?P<trait>[A-Za-z_][A-Za-z0-9_:]*)\s+for\s+)?(?P<ty>[A-Za-z_][A-Za-z0-9_:]*)",
    )?;
    let mut out = Vec::new();

    for file in iter_source_files(root, &["rs"]) {
        let rel = file.strip_prefix(root).unwrap_or(&file);
        let text = std::fs::read_to_string(&file)?;
        let test_ranges = cfg_test_ranges(&text);
        let module = module_qualname(rel);
        let is_test_file = rel.components().any(|c| c.as_os_str() == "tests")
            || file
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with("_test.rs") || n.starts_with("test_"));

        for caps in struct_re.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let vis = caps.name("vis").map(|m| m.as_str()).unwrap_or("");
            let is_public = vis.contains("pub");
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            let kind = caps.get(0).unwrap().as_str();
            let kind = if kind.contains("struct") {
                "struct"
            } else if kind.contains("enum") {
                "enum"
            } else if kind.contains("trait") {
                "trait"
            } else {
                "type"
            };
            let in_test = test_ranges.iter().any(|(a, b)| line >= *a && line <= *b);
            out.push(
                Symbol {
                    name: name.to_string(),
                    kind: if in_test || is_test_file {
                        "test".into()
                    } else {
                        kind.into()
                    },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line,
                    signature: caps.get(0).unwrap().as_str().trim().to_string(),
                    body: String::new(),
                    is_public,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}::{name}"),
                    visibility: visibility_label(vis, is_public),
                    parent: None,
                    docstring: leading_doc(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
        }

        for caps in const_re.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let vis = caps.name("vis").map(|m| m.as_str()).unwrap_or("");
            let is_public = vis.contains("pub");
            if !is_public {
                continue;
            }
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            out.push(
                Symbol {
                    name: name.to_string(),
                    kind: "const".into(),
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line: line,
                    signature: caps.get(0).unwrap().as_str().trim().to_string(),
                    body: String::new(),
                    is_public: true,
                    uses_io: false,
                    branch_score: 1,
                    qualname: format!("{module}::{name}"),
                    visibility: visibility_label(vis, true),
                    parent: None,
                    docstring: leading_doc(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
        }

        let mut impl_regions: Vec<(usize, usize, String)> = Vec::new();
        for caps in impl_re.captures_iter(&text) {
            let ty = caps.name("ty").unwrap().as_str();
            let start = caps.get(0).unwrap().start();
            let body_start = caps.get(0).unwrap().end();
            let body = extract_fn_body(&text, body_start);
            let end = body_start + body.len();
            let ty_name = ty.rsplit("::").next().unwrap_or(ty);
            impl_regions.push((start, end, ty_name.to_string()));
        }

        for caps in fn_re.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let vis = caps.name("vis").map(|m| m.as_str()).unwrap_or("");
            let is_public = vis.contains("pub");
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            let in_test = test_ranges.iter().any(|(a, b)| line >= *a && line <= *b)
                || name.starts_with("test_")
                || name.ends_with("_test")
                || is_test_file;

            let parent = impl_regions
                .iter()
                .find(|(a, b, _)| start >= *a && start <= *b)
                .map(|(_, _, ty)| ty.clone());

            if !is_public && !in_test && parent.is_none() {
                continue;
            }

            if !is_public && parent.is_none() && !in_test {
                continue;
            }
            if !is_public && parent.is_some() && !in_test {
            } else if !is_public && !in_test {
                continue;
            }

            let sig_rest = caps.name("sig").unwrap().as_str().trim();
            let signature = if is_public {
                format!("pub fn {name}{sig_rest}")
            } else {
                format!("fn {name}{sig_rest}")
            };
            let body = extract_fn_body(&text, caps.get(0).unwrap().end());
            let end_line = line + body.lines().count().saturating_sub(1).max(1);
            let uses_io = looks_like_io(&signature, &body);
            let branch_score = count_branches(&body);
            let qualname = if let Some(ref p) = parent {
                format!("{module}::{p}::{name}")
            } else {
                format!("{module}::{name}")
            };

            out.push(
                Symbol {
                    name: name.to_string(),
                    kind: if in_test { "test".into() } else { "fn".into() },
                    file: rel.to_path_buf(),
                    start_line: line,
                    end_line,
                    signature,
                    body,
                    is_public,
                    uses_io,
                    branch_score,
                    qualname,
                    visibility: visibility_label(vis, is_public),
                    parent,
                    docstring: leading_doc(&text, start),
                    start_byte: start,
                }
                .finalize(),
            );
        }
    }
    Ok(out)
}

fn module_qualname(rel: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in rel.components() {
        let s = c.as_os_str().to_string_lossy();
        if s == "src" {
            continue;
        }
        if let Some(stem) = s.strip_suffix(".rs") {
            if stem != "mod" && stem != "lib" && stem != "main" {
                parts.push(stem.replace('-', "_"));
            }
        } else {
            parts.push(s.replace('-', "_"));
        }
    }
    if parts.is_empty() {
        "crate".into()
    } else {
        format!("crate::{}", parts.join("::"))
    }
}

fn visibility_label(vis: &str, is_public: bool) -> String {
    if vis.contains("pub(crate)") {
        "crate".into()
    } else if vis.contains("pub(") {
        "restricted".into()
    } else if is_public || vis.contains("pub") {
        "pub".into()
    } else {
        "private".into()
    }
}

fn leading_doc(text: &str, byte_idx: usize) -> String {
    let before = &text[..byte_idx];
    let mut docs = Vec::new();
    for line in before.lines().rev().take(8) {
        let t = line.trim();
        if t.starts_with("///") || t.starts_with("//!") {
            docs.push(t.trim_start_matches('/').trim().to_string());
        } else if t.starts_with("#[") || t.is_empty() {
            continue;
        } else {
            break;
        }
    }
    docs.reverse();
    docs.into_iter().take(3).collect::<Vec<_>>().join(" ")
}

fn cfg_test_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("#[cfg(test)]") {
            let mut j = i + 1;
            while j < lines.len() && !lines[j].contains('{') {
                j += 1;
            }
            if j >= lines.len() {
                break;
            }
            let mut depth = 0;
            let start = i + 1;
            for k in j..lines.len() {
                depth += lines[k].matches('{').count();
                depth = depth.saturating_sub(lines[k].matches('}').count());
                if depth == 0 && k > j {
                    ranges.push((start, k + 1));
                    i = k + 1;
                    break;
                }
            }
            continue;
        }
        i += 1;
    }
    ranges
}

fn byte_line(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].bytes().filter(|b| *b == b'\n').count() + 1
}

fn extract_fn_body(text: &str, start_after_sig: usize) -> String {
    let rest = &text[start_after_sig..];
    let brace = match rest.find('{') {
        Some(i) => i,
        None => return String::new(),
    };
    let mut depth = 0usize;
    let bytes = rest.as_bytes();
    let mut end = brace;
    for i in brace..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    rest[brace..end].to_string()
}

fn looks_like_io(signature: &str, body: &str) -> bool {
    let blob = format!("{signature}\n{body}");
    const NEEDLES: &[&str] = &[
        "std::fs",
        "std::net",
        "TcpListener",
        "TcpStream",
        "UdpSocket",
        "reqwest",
        "hyper",
        "axum",
        "actix",
        "sqlx",
        "diesel",
        "tokio::net",
        "Path",
        "read_to_string",
        "File::",
        "OpenOptions",
    ];
    NEEDLES.iter().any(|n| blob.contains(n))
}

fn count_branches(body: &str) -> u32 {
    let keys = [
        " if ", "\nif ", " else", " match ", "\nmatch ", " for ", " while ",
    ];
    keys.iter()
        .map(|k| body.matches(k).count() as u32)
        .sum::<u32>()
        .max(1)
}
