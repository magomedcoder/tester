use crate::index::iter_source_files;
use crate::model::Symbol;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

pub fn index_rust(root: &Path) -> Result<Vec<Symbol>> {
    let fn_re = Regex::new(
        r"(?m)^(?P<indent>\s*)(?P<vis>pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<sig>\([^;{]*)",
    )?;
    let mut out = Vec::new();

    for file in iter_source_files(root, &["rs"]) {
        let rel = file.strip_prefix(root).unwrap_or(&file);
        if rel.components().any(|c| c.as_os_str() == "tests") {
            continue;
        }
        let text = std::fs::read_to_string(&file)?;
        let test_ranges = cfg_test_ranges(&text);

        for caps in fn_re.captures_iter(&text) {
            let name = caps.name("name").unwrap().as_str();
            let vis = caps.name("vis").map(|m| m.as_str()).unwrap_or("");
            let is_public = vis.contains("pub");
            if !is_public {
                continue;
            }
            let start = caps.get(0).unwrap().start();
            let line = byte_line(&text, start);
            if test_ranges.iter().any(|(a, b)| line >= *a && line <= *b) {
                continue;
            }
            if name.starts_with("test_") || name.ends_with("_test") {
                continue;
            }

            let sig_rest = caps.name("sig").unwrap().as_str().trim();
            let signature = format!("pub fn {name}{sig_rest}");
            let body = extract_fn_body(&text, caps.get(0).unwrap().end());
            let end_line = line + body.lines().count().saturating_sub(1).max(1);
            let uses_io = looks_like_io(&signature, &body);
            let branch_score = count_branches(&body);

            out.push(Symbol {
                name: name.to_string(),
                kind: "fn".into(),
                file: rel.to_path_buf(),
                start_line: line,
                end_line,
                signature,
                body,
                is_public: true,
                uses_io,
                branch_score,
            });
        }
    }
    Ok(out)
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
