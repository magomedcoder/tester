pub mod rust;
pub mod typescript;

use crate::adapter;
use crate::cache;
use crate::model::{ProjectProfile, Symbol};
use crate::security::skip_dir_name;
use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

pub fn index_symbols(profile: &ProjectProfile) -> Result<Vec<Symbol>> {
    if let Some(cached) = cache::load_index(profile)? {
        return Ok(cached);
    }
    let adapter = adapter::adapter_for(profile.language)?;
    let mut symbols = adapter.index(profile)?;
    symbols.sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));
    cache::save_index(profile, &symbols)?;
    Ok(symbols)
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
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"multiply"));
        assert!(names.contains(&"add"));
        assert!(names.contains(&"read_config"));
        assert!(!names.iter().any(|n| n.contains("add_works")));
    }
}
