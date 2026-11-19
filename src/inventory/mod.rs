use crate::adapter;
use crate::index::iter_source_files;
use crate::model::{ProjectProfile, TestInventory, TestInventoryItem};
use anyhow::Result;
use regex::Regex;
use std::path::Path;

pub fn collect_inventory(profile: &ProjectProfile) -> Result<TestInventory> {
    let adapter = adapter::adapter_for(profile.language)?;
    let mut items = Vec::new();
    for file in iter_source_files(&profile.root, adapter.inventory_globs()) {
        let text = std::fs::read_to_string(&file)?;
        if !adapter.is_test_file(&file, &text) {
            continue;
        }
        let rel = file
            .strip_prefix(&profile.root)
            .unwrap_or(&file)
            .to_path_buf();
        items.push(parse_generic_tests(&rel, &text)?);
    }
    Ok(TestInventory { items })
}

fn parse_generic_tests(path: &Path, text: &str) -> Result<TestInventoryItem> {
    let mut names = Vec::new();
    let fn_re = Regex::new(r"(?m)^\s*(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(")?;
    let go_re = Regex::new(r"(?m)^func\s+(Test[A-Za-z0-9_]*)\s*\(")?;
    let title_re = Regex::new(r#"(?:it|test|describe|TEST|TEST_F|TEST_CASE)\(\s*['"]?([^'"\)]+)"#)?;
    let py_re = Regex::new(r"(?m)^def\s+(test_[A-Za-z0-9_]*)\s*\(")?;
    let php_re = Regex::new(r"(?m)^\s*(?:public\s+)?function\s+(test[A-Za-z0-9_]*)\s*\(")?;
    let kt_re = Regex::new(r#"(?m)@Test\s+(?:fun\s+)?[`']?([A-Za-z0-9_ ]+)[`']?"#)?;

    for caps in fn_re.captures_iter(text) {
        names.push(caps.name("name").unwrap().as_str().to_string());
    }
    for caps in go_re.captures_iter(text) {
        names.push(caps.get(1).unwrap().as_str().to_string());
    }
    for caps in title_re.captures_iter(text) {
        names.push(caps.get(1).unwrap().as_str().trim().to_string());
    }
    for caps in py_re.captures_iter(text) {
        names.push(caps.get(1).unwrap().as_str().to_string());
    }
    for caps in php_re.captures_iter(text) {
        names.push(caps.get(1).unwrap().as_str().to_string());
    }
    for caps in kt_re.captures_iter(text) {
        names.push(caps.get(1).unwrap().as_str().replace(' ', "_"));
    }
    names.sort();
    names.dedup();

    Ok(TestInventoryItem {
        path: path.to_path_buf(),
        names,
        mentioned_symbols: mention_candidates(path, text),
    })
}

fn mention_candidates(path: &Path, text: &str) -> Vec<String> {
    let region = if path.extension().and_then(|e| e.to_str()) == Some("rs")
        && text.contains("#[cfg(test)]")
        && !path.components().any(|c| c.as_os_str() == "tests")
    {
        rust_test_region(text)
    } else {
        text.to_string()
    };
    let ident = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\b").unwrap();
    let mut out = Vec::new();
    for caps in ident.captures_iter(&region) {
        let w = caps.get(1).unwrap().as_str();
        if w.len() >= 3 {
            out.push(w.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn rust_test_region(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
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
            let mut depth = 0usize;
            for k in j..lines.len() {
                depth += lines[k].matches('{').count();
                depth = depth.saturating_sub(lines[k].matches('}').count());
                out.push_str(lines[k]);
                out.push('\n');
                if depth == 0 && k > j {
                    i = k + 1;
                    break;
                }
            }
            continue;
        }
        i += 1;
    }
    out
}

pub fn covers_symbol(inventory: &TestInventory, symbol_name: &str) -> bool {
    let lower = symbol_name.to_ascii_lowercase();
    inventory.items.iter().any(|item| {
        item.names.iter().any(|n| {
            let nl = n.to_ascii_lowercase();
            nl == lower || nl.contains(&lower)
        }) || item.mentioned_symbols.iter().any(|m| m == symbol_name)
            || item
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| {
                    let stem = stem
                        .trim_end_matches(".test")
                        .trim_end_matches(".spec")
                        .trim_end_matches("_test")
                        .trim_end_matches("Test")
                        .replace('-', "_");
                    stem.eq_ignore_ascii_case(symbol_name)
                })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::discover;
    use std::path::Path;

    fn write_rust(dir: &Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"d\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_works() { assert_eq!(add(2, 3), 5); }
}
"#,
        )
        .unwrap();
    }

    #[test]
    fn finds_add_coverage() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        let inv = collect_inventory(&profile).unwrap();
        assert!(covers_symbol(&inv, "add"));
        assert!(!covers_symbol(&inv, "multiply"));
    }
}
