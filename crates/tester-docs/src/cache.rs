use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub path: String,
    pub kind: String,
    pub source: String,
    pub engine: String,
    pub mtime_secs: u64,
    pub size: u64,
}

pub fn docs_dir(project_root: &Path) -> PathBuf {
    project_root.join(".tester").join("docs")
}

pub fn ensure_docs_dir(project_root: &Path) -> Result<PathBuf> {
    let dir = docs_dir(project_root);
    fs::create_dir_all(dir.join("cache")).with_context(|| i18n::io_ctx(Msg::IoCreate, &dir))?;
    fs::create_dir_all(dir.join("converted"))
        .with_context(|| i18n::io_ctx(Msg::IoCreate, &dir))?;
    Ok(dir)
}

pub fn cache_key(path: &Path, mtime_secs: u64, size: u64, engine: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut h);
    mtime_secs.hash(&mut h);
    size.hash(&mut h);
    engine.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn file_stamp(path: &Path) -> Result<(u64, u64)> {
    let meta = fs::metadata(path).with_context(|| i18n::io_ctx(Msg::IoRead, path))?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok((mtime, meta.len()))
}

pub fn read_cached(project_root: &Path, key: &str) -> Result<Option<(CacheMeta, String)>> {
    let dir = docs_dir(project_root).join("cache").join(key);
    let meta_path = dir.join("meta.json");
    let text_path = dir.join("text.txt");
    if !meta_path.is_file() || !text_path.is_file() {
        return Ok(None);
    }
    let meta: CacheMeta = serde_json::from_str(
        &fs::read_to_string(&meta_path).with_context(|| i18n::io_ctx(Msg::IoRead, &meta_path))?,
    )?;
    let text =
        fs::read_to_string(&text_path).with_context(|| i18n::io_ctx(Msg::IoRead, &text_path))?;
    Ok(Some((meta, text)))
}

pub fn write_cached(
    project_root: &Path,
    key: &str,
    meta: &CacheMeta,
    text: &str,
) -> Result<()> {
    ensure_docs_dir(project_root)?;
    let dir = docs_dir(project_root).join("cache").join(key);
    fs::create_dir_all(&dir).with_context(|| i18n::io_ctx(Msg::IoCreate, &dir))?;
    let meta_path = dir.join("meta.json");
    let text_path = dir.join("text.txt");
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(meta)?,
    )
    .with_context(|| i18n::io_ctx(Msg::IoWrite, &meta_path))?;
    fs::write(&text_path, text).with_context(|| i18n::io_ctx(Msg::IoWrite, &text_path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_stable() {
        let a = cache_key(Path::new("/tmp/a.pdf"), 100, 50, "pdftotext");
        let b = cache_key(Path::new("/tmp/a.pdf"), 100, 50, "pdftotext");
        let c = cache_key(Path::new("/tmp/a.pdf"), 101, 50, "pdftotext");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn roundtrip_cache() {
        let home = tempfile::tempdir().unwrap();
        let key = "abc123";
        let meta = CacheMeta {
            path: "x.pdf".into(),
            kind: "pdf".into(),
            source: "extract".into(),
            engine: "pdftotext".into(),
            mtime_secs: 1,
            size: 2,
        };
        write_cached(home.path(), key, &meta, "hello").unwrap();
        let (m, t) = read_cached(home.path(), key).unwrap().unwrap();
        assert_eq!(t, "hello");
        assert_eq!(m.engine, "pdftotext");
    }
}
