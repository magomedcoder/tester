use std::path::Path;
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

pub fn list_project_files(root: &Path, needle: &str) -> Vec<String> {
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
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
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

pub fn read_attach_snippet(root: &Path, spec: &str) -> Option<(String, String)> {
    let (path_part, range) = match spec.split_once('#') {
        Some((p, r)) => (p, Some(r)),
        None => (spec, None),
    };
    let path_part = path_part.trim().trim_start_matches('@');
    if path_part.is_empty() {
        return None;
    }
    let full = root.join(path_part);
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
