use std::path::{Component, Path};

pub fn path_escapes_root(root: &Path, candidate: &Path) -> bool {
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };

    let Ok(root_c) = root.canonicalize() else {
        return has_parent_component(candidate);
    };

    match joined.canonicalize() {
        Ok(c) => !c.starts_with(&root_c),
        Err(_) => {
            has_parent_component(candidate)
                || candidate
                    .is_absolute()
                    .then(|| !candidate.starts_with(root))
                    .unwrap_or(false)
        }
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

pub fn skip_dir_name(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | ".git"
            | "dist"
            | "build"
            | ".next"
            | "coverage"
            | ".tester"
            | "vendor"
            | ".venv"
            | "venv"
            | "__pycache__"
            | "cmake-build-debug"
            | "cmake-build-release"
            | ".idea"
            | ".gradle"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_parent_paths() {
        let dir = tempfile::tempdir().unwrap();
        assert!(path_escapes_root(dir.path(), Path::new("../outside.rs")));
        let inside = Path::new("tests/foo.rs");
        assert!(!path_escapes_root(dir.path(), inside));
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        assert!(!path_escapes_root(dir.path(), inside));
    }
}
