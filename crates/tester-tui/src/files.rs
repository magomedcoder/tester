use std::path::Path;
use tester_core::model::Symbol;
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    ".tester",
    "vendor",
    ".idea",
    ".vscode",
];

fn list_project_files(root: &Path, needle: &str) -> Vec<String> {
    let needle = needle.to_lowercase();
    let mut out = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.iter().any(|s| *s == name)
        });
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            if needle.is_empty() || rel.to_lowercase().contains(&needle) {
                if entry.depth() > 0 && entry.depth() <= 4 {
                    out.push(format!("{rel}/"));
                }
            }
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if needle.is_empty() || rel.to_lowercase().contains(&needle) {
            out.push(rel);
        }
        if out.len() >= 200 {
            break;
        }
    }
    out.sort();
    out.truncate(80);
    out
}

pub fn list_at_candidates(root: &Path, needle: &str, symbols: &[Symbol]) -> Vec<String> {
    let mut out = list_project_files(root, needle);
    let needle_l = needle.to_lowercase();
    let want_symbols =
        needle.contains("::") || needle.contains('.') || !needle.contains('/') || needle.is_empty();
    if want_symbols {
        let mut sym_out: Vec<(f64, String)> = Vec::new();
        for s in symbols {
            let name = if s.qualname.is_empty() {
                s.name.clone()
            } else {
                s.qualname.clone()
            };
            let name_l = name.to_lowercase();
            let short_l = s.name.to_lowercase();
            let score = if needle.is_empty() {
                0.5
            } else if short_l == needle_l || name_l == needle_l {
                100.0
            } else if short_l.starts_with(&needle_l) || name_l.starts_with(&needle_l) {
                70.0
            } else if short_l.contains(&needle_l) || name_l.contains(&needle_l) {
                40.0
            } else {
                continue;
            };
            sym_out.push((score, format!("Symbol:{name}")));
        }
        sym_out.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, s) in sym_out.into_iter().take(40) {
            out.push(s);
        }
    }
    out.truncate(80);
    out
}

pub fn read_attach_snippet(root: &Path, spec: &str) -> Option<(String, String)> {
    let spec = spec.trim().trim_start_matches('@');

    if spec.ends_with('/') {
        return read_dir_listing(root, spec.trim_end_matches('/'));
    }

    if let Some(name) = spec.strip_prefix("Symbol:") {
        return resolve_symbol_attach(root, name);
    }
    if !spec.contains('/') && !spec.contains('\\') && !spec.contains('#') {
        if let Some(hit) = resolve_symbol_attach(root, spec) {
            return Some(hit);
        }
    }

    let (path_part, range) = match spec.split_once('#') {
        Some((p, r)) => (p, Some(r)),
        None => (spec, None),
    };
    let path_part = path_part.trim();
    if path_part.is_empty() {
        return None;
    }
    let full = root.join(path_part);
    if full.is_dir() {
        return read_dir_listing(root, path_part);
    }
    let text = std::fs::read_to_string(&full).ok()?;
    let body = if let Some(r) = range {
        let r = r.trim().trim_start_matches('L').trim_start_matches('l');
        let (a, b) = r.split_once('-').unwrap_or((r, r));
        let start: usize = a.parse::<usize>().unwrap_or(1).saturating_sub(1);
        let end: usize = b.parse::<usize>().unwrap_or(start + 1);
        text.lines()
            .skip(start)
            .take(end.saturating_sub(start).max(1))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let mut s = String::new();
        for (i, line) in text.lines().enumerate() {
            if i >= 200 {
                s.push_str("\n... truncated");
                break;
            }
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(line);
        }
        s
    };
    Some((path_part.to_string(), body))
}

fn read_dir_listing(root: &Path, rel: &str) -> Option<(String, String)> {
    let abs = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    if !abs.is_dir() {
        return None;
    }
    let mut lines = Vec::new();
    let walker = WalkDir::new(&abs)
        .follow_links(false)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.iter().any(|s| *s == name)
        });
    for entry in walker.filter_map(|e| e.ok()).take(80) {
        let Ok(r) = entry.path().strip_prefix(root) else {
            continue;
        };
        let s = r.to_string_lossy().replace('\\', "/");
        if s.is_empty() {
            continue;
        }
        if entry.file_type().is_dir() {
            lines.push(format!("{s}/"));
        } else {
            lines.push(s);
        }
    }
    Some((format!("{rel}/"), lines.join("\n")))
}

fn resolve_symbol_attach(root: &Path, name: &str) -> Option<(String, String)> {
    tester_agent::facade::resolve_symbol_attach(root, name)
}

pub fn extract_at_specs(composer: &str) -> Vec<String> {
    let mut specs = Vec::new();
    for token in composer.split_whitespace() {
        if let Some(rest) = token.strip_prefix('@') {
            if !rest.is_empty() {
                specs.push(rest.to_string());
            }
        }
    }
    specs
}

pub fn active_at_needle(composer: &str) -> Option<String> {
    let trimmed = composer;
    if let Some(idx) = trimmed.rfind('@') {
        let after = &trimmed[idx + 1..];
        if after.contains(char::is_whitespace) {
            return None;
        }

        if idx > 0 {
            let before = trimmed.as_bytes()[idx - 1];
            if !before.is_ascii_whitespace() {
                return None;
            }
        }
        return Some(after.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_filters_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "fn x(){}\n").unwrap();
        std::fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        std::fs::write(dir.path().join("target/debug/x"), "bin").unwrap();
        let files = list_project_files(dir.path(), "lib");
        assert!(files.iter().any(|f| f.contains("lib.rs")), "{files:?}");
        assert!(!files.iter().any(|f| f.contains("target")), "{files:?}");
    }

    #[test]
    fn active_at_needle_works() {
        assert_eq!(active_at_needle("see @src/li").as_deref(), Some("src/li"));
        assert_eq!(active_at_needle("see @src/lib.rs more"), None);
        assert_eq!(active_at_needle("nope"), None);
    }

    #[test]
    fn extract_specs() {
        let s = extract_at_specs("review @src/a.rs and @b.ts#L1-3");
        assert_eq!(s, vec!["src/a.rs", "b.ts#L1-3"]);
    }
}
