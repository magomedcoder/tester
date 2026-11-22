pub mod rust;
pub mod typescript;

use crate::adapter;
use crate::cache;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tester_core::model::{ProjectProfile, Symbol};
use tester_core::security::skip_dir_name;
use walkdir::WalkDir;

pub fn index_symbols(profile: &ProjectProfile) -> Result<Vec<Symbol>> {
    if let Some(cached) = cache::load_index(profile)? {
        return Ok(cached);
    }
    let adapter = adapter::adapter_for(profile.language)?;
    let mut symbols = adapter.index(profile)?;
    symbols = symbols.into_iter().map(|s| s.finalize()).collect();
    symbols.sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));
    cache::save_index(profile, &symbols)?;
    Ok(symbols)
}

pub fn rebuild_index(profile: &ProjectProfile) -> Result<Vec<Symbol>> {
    cache::invalidate(&profile.root)?;
    index_symbols(profile)
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStatus {
    pub symbol_count: usize,
    pub stale: bool,
    pub git_head: Option<String>,
    pub current_head: Option<String>,
    pub cache_exists: bool,
    pub version: Option<u32>,
    pub by_kind: std::collections::BTreeMap<String, usize>,
}

pub fn index_status(profile: &ProjectProfile) -> Result<IndexStatus> {
    let cache_st = cache::index_cache_status(&profile.root)?;
    let symbols = if cache_st.exists && !cache_st.stale {
        cache::load_index(profile)?.unwrap_or_default()
    } else if cache_st.exists {
        cache::load_index(profile)?.unwrap_or_else(Vec::new)
    } else {
        Vec::new()
    };
    let symbols = if symbols.is_empty() && cache_st.symbol_count > 0 {
        let path = profile.root.join(".tester").join("index-cache.json");
        if path.exists() {
            #[derive(serde::Deserialize)]
            struct Raw {
                symbols: Vec<Symbol>,
            }
            serde_json::from_str::<Raw>(&std::fs::read_to_string(&path)?)?.symbols
        } else {
            symbols
        }
    } else {
        symbols
    };
    let mut by_kind = std::collections::BTreeMap::new();
    for s in &symbols {
        *by_kind.entry(s.kind.clone()).or_insert(0usize) += 1;
    }
    Ok(IndexStatus {
        symbol_count: symbols.len().max(cache_st.symbol_count),
        stale: cache_st.stale,
        git_head: cache_st.git_head,
        current_head: cache_st.current_head,
        cache_exists: cache_st.exists,
        version: cache_st.version,
        by_kind,
    })
}

pub fn iter_source_files(root: &Path, extensions: &[&str]) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            !e.path()
                .components()
                .any(|c| c.as_os_str().to_str().is_some_and(skip_dir_name))
        })
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            if name.ends_with(".min.js") || name.contains(".generated.") {
                return false;
            }
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
        })
        .map(|e| e.path().to_path_buf())
        .collect()
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
pub fn read_config(path: &std::path::Path) -> std::io::Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}
pub struct Foo;
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
    fn indexes_rust_public_fns() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        let symbols = index_symbols(&profile).unwrap();
        let prod: Vec<_> = symbols.iter().filter(|s| s.kind != "test").collect();
        let names: Vec<_> = prod.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"multiply"));
        assert!(names.contains(&"add"));
        assert!(names.contains(&"read_config"));
        assert!(names.contains(&"Foo"));
        let tests: Vec<_> = symbols.iter().filter(|s| s.kind == "test").collect();
        assert!(tests.iter().any(|s| s.name == "add_works"));
    }
}
