use std::path::{Path, PathBuf};
use tester_core::security::skip_dir_name;
use walkdir::WalkDir;

pub fn iter_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str().to_str().is_some_and(skip_dir_name))
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if name.ends_with(".min.js") || name.ends_with(".map") || name.ends_with(".lock") {
            if !name.ends_with("Cargo.lock")
                && !name.ends_with("composer.lock")
                && name != "package-lock.json"
                && name != "pnpm-lock.yaml"
                && name != "go.sum"
                && name != "uv.lock"
            {
                if name.ends_with(".lock") || name.ends_with(".map") || name.ends_with(".min.js") {
                    continue;
                }
            }
        }
        out.push(entry.path().to_path_buf());
        if out.len() >= max_files {
            break;
        }
    }
    out
}

pub fn rel(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

pub fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}
