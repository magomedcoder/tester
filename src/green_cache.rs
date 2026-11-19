use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct GreenCache {
    symbols: BTreeSet<String>,
}

fn path(root: &Path) -> PathBuf {
    root.join(".tester").join("green-cache.json")
}

pub fn is_green(root: &Path, symbol: &str) -> bool {
    load(root)
        .map(|c| c.symbols.contains(symbol))
        .unwrap_or(false)
}

pub fn mark_green(root: &Path, symbol: &str) -> Result<()> {
    let mut c = load(root).unwrap_or_default();
    c.symbols.insert(symbol.to_string());
    save(root, &c)
}

pub fn clear(root: &Path) -> Result<()> {
    let p = path(root);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

fn load(root: &Path) -> Result<GreenCache> {
    let text = std::fs::read_to_string(path(root))?;
    Ok(serde_json::from_str(&text)?)
}

fn save(root: &Path, cache: &GreenCache) -> Result<()> {
    let p = path(root);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_string_pretty(cache)?)?;
    Ok(())
}
