use crate::adapter;
use anyhow::Result;
use std::path::Path;
use tester_core::model::ProjectProfile;

pub fn discover(path: &Path) -> Result<ProjectProfile> {
    let root = adapter::find_project_root(path)?;
    let root = root.canonicalize().unwrap_or(root);
    adapter::discover(&root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tester_core::model::Language;

    fn write_rust(dir: &Path) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"d\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[lib]\npath=\"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn add(a:i32,b:i32)->i32{a+b}\n",
        )
        .unwrap();
    }

    fn write_ts(dir: &Path) {
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"ts_demo","devDependencies":{"vitest":"^2.1.0"}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/math.ts"),
            "export function add(a:number,b:number){return a+b}\n",
        )
        .unwrap();
    }

    #[test]
    fn discovers_rust_fixture() {
        let dir = tempfile::tempdir().unwrap();
        write_rust(dir.path());
        let profile = discover(dir.path()).unwrap();
        assert_eq!(profile.language, Language::Rust);
        assert!(profile.supported);
        assert_eq!(profile.test_command, "cargo test");
        assert!(profile.capabilities.unit);
    }

    #[test]
    fn discovers_ts_fixture() {
        let dir = tempfile::tempdir().unwrap();
        write_ts(dir.path());
        let profile = discover(dir.path()).unwrap();
        assert!(matches!(
            profile.language,
            Language::TypeScript | Language::JavaScript
        ));
        assert_eq!(profile.unit_framework.as_deref(), Some("vitest"));
    }
}
