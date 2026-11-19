use crate::model::{ProjectProfile, Symbol};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const INDEX_CACHE_VERSION: u32 = 2;

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

fn fingerprint(root: &Path) -> Result<BTreeMap<String, FileMeta>> {
    let mut map = BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some(
                    "target"
                        | "node_modules"
                        | ".git"
                        | ".tester"
                        | "vendor"
                        | "build"
                        | ".venv"
                        | "dist"
                )
            )
        }) {
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

pub fn load_index(profile: &ProjectProfile) -> Result<Option<Vec<Symbol>>> {
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
    Ok(None)
}

pub fn save_index(profile: &ProjectProfile, symbols: &[Symbol]) -> Result<()> {
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
    Ok(())
}
