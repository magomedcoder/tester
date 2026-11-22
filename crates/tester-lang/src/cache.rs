use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tester_core::model::{ProjectProfile, Symbol};
use walkdir::WalkDir;

const INDEX_CACHE_VERSION: u32 = 3;

#[derive(Debug, Serialize, Deserialize)]
struct IndexCache {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    git_head: Option<String>,
    fingerprint: BTreeMap<String, FileMeta>,
    symbols: Vec<Symbol>,
}

fn git_head(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileMeta {
    len: u64,
    modified: u64,
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(".tester").join("index-cache.json")
}

fn should_skip_component(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | ".git"
            | ".tester"
            | "vendor"
            | "build"
            | ".venv"
            | "dist"
            | "generated"
    ) || name.ends_with(".min.js")
}

fn fingerprint(root: &Path) -> Result<BTreeMap<String, FileMeta>> {
    let mut map = BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path
            .components()
            .any(|c| c.as_os_str().to_str().is_some_and(should_skip_component))
        {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.ends_with(".min.js") || name.ends_with(".lock") {
            continue;
        }
        let meta = entry.metadata()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        map.insert(
            rel,
            FileMeta {
                len: meta.len(),
                modified,
            },
        );
    }
    Ok(map)
}

pub(crate) fn load_index(profile: &ProjectProfile) -> Result<Option<Vec<Symbol>>> {
    let path = cache_path(&profile.root);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    let cached: IndexCache = serde_json::from_str(&text)?;
    if cached.version != INDEX_CACHE_VERSION {
        return Ok(None);
    }
    let head = git_head(&profile.root);
    if cached.git_head != head {
        return Ok(None);
    }
    let current = fingerprint(&profile.root)?;
    if cached.fingerprint == current {
        return Ok(Some(cached.symbols));
    }
    if let Some(merged) = try_incremental(profile, &cached, &current)? {
        return Ok(Some(merged));
    }
    Ok(None)
}

fn try_incremental(
    profile: &ProjectProfile,
    cached: &IndexCache,
    current: &BTreeMap<String, FileMeta>,
) -> Result<Option<Vec<Symbol>>> {
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    for (path, meta) in current {
        match cached.fingerprint.get(path) {
            Some(old) if old == meta => {}
            _ => changed.push(path.clone()),
        }
    }
    for path in cached.fingerprint.keys() {
        if !current.contains_key(path) {
            removed.push(path.clone());
        }
    }

    if changed.len() + removed.len() > 40 || changed.len() + removed.len() == 0 {
        return Ok(None);
    }

    let mut symbols: Vec<Symbol> = cached
        .symbols
        .iter()
        .filter(|s| {
            let f = s.file.to_string_lossy();
            !changed.iter().any(|c| c == &f) && !removed.iter().any(|c| c == &f)
        })
        .cloned()
        .collect();

    let adapter = crate::adapter::adapter_for(profile.language)?;
    let fresh = adapter.index(profile)?;
    for path in &changed {
        symbols.retain(|s| s.file.to_string_lossy() != path.as_str());
        for s in fresh
            .iter()
            .filter(|s| s.file.to_string_lossy() == path.as_str())
        {
            symbols.push(s.clone());
        }
    }
    symbols.sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));
    save_index(profile, &symbols)?;
    Ok(Some(symbols))
}

pub(crate) fn save_index(profile: &ProjectProfile, symbols: &[Symbol]) -> Result<()> {
    let path = cache_path(&profile.root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cached = IndexCache {
        version: INDEX_CACHE_VERSION,
        git_head: git_head(&profile.root),
        fingerprint: fingerprint(&profile.root)?,
        symbols: symbols.to_vec(),
    };
    std::fs::write(path, serde_json::to_string_pretty(&cached)?)?;
    Ok(())
}

pub fn invalidate(root: &Path) -> Result<()> {
    let path = cache_path(root);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let graph = root.join(".tester").join("graph").join("graph.json");
    if graph.exists() {
        let _ = std::fs::remove_file(graph);
    }
    let map = root.join(".tester").join("map.json");
    if map.exists() {
        let _ = std::fs::remove_file(map);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStatus {
    pub exists: bool,
    pub version: Option<u32>,
    pub expected_version: u32,
    pub symbol_count: usize,
    pub git_head: Option<String>,
    pub current_head: Option<String>,
    pub stale: bool,
    pub fingerprint_files: usize,
}

pub fn index_cache_status(root: &Path) -> Result<CacheStatus> {
    let path = cache_path(root);
    let current_head = git_head(root);
    if !path.exists() {
        return Ok(CacheStatus {
            exists: false,
            version: None,
            expected_version: INDEX_CACHE_VERSION,
            symbol_count: 0,
            git_head: None,
            current_head,
            stale: true,
            fingerprint_files: 0,
        });
    }
    let text = std::fs::read_to_string(&path)?;
    let cached: IndexCache = serde_json::from_str(&text)?;
    let current = fingerprint(root)?;
    let stale = cached.version != INDEX_CACHE_VERSION
        || cached.git_head != current_head
        || cached.fingerprint != current;
    Ok(CacheStatus {
        exists: true,
        version: Some(cached.version),
        expected_version: INDEX_CACHE_VERSION,
        symbol_count: cached.symbols.len(),
        git_head: cached.git_head,
        current_head,
        stale,
        fingerprint_files: cached.fingerprint.len(),
    })
}
