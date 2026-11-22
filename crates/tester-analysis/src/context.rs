use super::AnalysisHit;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use tester_core::model::Symbol;

pub fn expand_context(
    root: &Path,
    symbols: &[Symbol],
    path: &str,
    line: usize,
    pad: usize,
) -> Result<String> {
    let abs = root.join(path);
    if !abs.exists() {
        bail!("файл не найден: {path}");
    }
    let text = std::fs::read_to_string(&abs)?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Ok(String::new());
    }
    let line = line.max(1);
    let pad = pad.clamp(1, 80);
    let start = line.saturating_sub(pad).max(1);
    let end = (line + pad).min(lines.len());

    let file_path = PathBuf::from(path);
    let enclosing = symbols
        .iter()
        .filter(|s| {
            let f = s.file.to_string_lossy().replace('\\', "/");
            let p = path.replace('\\', "/");
            (f == p || f.ends_with(&p) || p.ends_with(&f))
                && s.start_line <= line
                && s.end_line >= line
        })
        .max_by_key(|s| s.start_line);

    let imports = extract_imports(&text);

    let mut out = String::new();
    out.push_str(&format!("path={path} line={line}\n"));
    if let Some(sym) = enclosing {
        out.push_str(&format!(
            "enclosing: {} ({}) L{}-L{}\n{}\n",
            if sym.qualname.is_empty() {
                &sym.name
            } else {
                &sym.qualname
            },
            sym.kind,
            sym.start_line,
            sym.end_line,
            sym.signature
        ));
    }
    if !imports.is_empty() {
        out.push_str("imports:\n");
        for i in imports.iter().take(30) {
            out.push_str(i);
            out.push('\n');
        }
    }
    out.push_str(&format!("--- L{start}-L{end} ---\n"));
    for (i, l) in lines
        .iter()
        .enumerate()
        .skip(start - 1)
        .take(end - start + 1)
    {
        let n = i + 1;
        let mark = if n == line { ">" } else { " " };
        out.push_str(&format!("{mark}{n:>5}| {l}\n"));
    }
    let _ = file_path;
    Ok(out)
}

fn extract_imports(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("use ")
                || t.starts_with("import ")
                || t.starts_with("from ")
                || t.starts_with("require(")
                || t.starts_with("#include")
                || t.starts_with("package ")
        })
        .take(40)
        .map(|l| l.to_string())
        .collect()
}

pub fn read_range(root: &Path, path: &str, start_line: usize, end_line: usize) -> Result<String> {
    let abs = root.join(path);
    let text = std::fs::read_to_string(&abs)?;
    let start = start_line.max(1);
    let end = end_line.max(start);
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if n < start {
            continue;
        }
        if n > end {
            break;
        }
        out.push_str(&format!("{n:>5}| {line}\n"));
    }
    Ok(out)
}

pub fn outline_as_text(hits: &[AnalysisHit]) -> String {
    if hits.is_empty() {
        return "(нет символов)".into();
    }
    let mut out = String::new();
    for h in hits {
        out.push_str(&format!(
            "L{}-L{} [{}] {} {}\n",
            h.start_line,
            h.end_line,
            h.kind.as_deref().unwrap_or("?"),
            h.symbol.as_deref().unwrap_or("?"),
            h.signature.as_deref().unwrap_or("")
        ));
    }
    out
}
