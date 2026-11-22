use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RECENTS: usize = 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Recents {
    pub paths: Vec<PathBuf>,
    #[serde(default)]
    pub last_root: Option<PathBuf>,
}

impl Recents {
    pub fn load() -> Self {
        Self::load_from(&store_path())
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        let mut recents: Recents = serde_json::from_str(&text).unwrap_or_default();
        recents.prune_missing();
        recents
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&store_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn push(&mut self, root: &Path) {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        self.paths.retain(|p| p != &root);
        self.paths.insert(0, root.clone());
        self.last_root = Some(root);
        if self.paths.len() > MAX_RECENTS {
            self.paths.truncate(MAX_RECENTS);
        }
    }

    pub fn prune_missing(&mut self) {
        self.paths.retain(|p| p.is_dir());
        if let Some(last) = &self.last_root {
            if !last.is_dir() {
                self.last_root = None;
            }
        }
    }
}

pub fn store_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join(".tester").join("recents.json");
    }
    dirs_home()
        .join(".local")
        .join("share")
        .join(".tester")
        .join("recents.json")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_dedups_and_caps() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        let mut r = Recents::default();
        r.push(&a);
        r.push(&b);
        r.push(&a);
        assert_eq!(r.paths[0], a.canonicalize().unwrap());
        assert_eq!(r.paths.len(), 2);
    }

    #[test]
    fn prune_drops_missing() {
        let mut r = Recents {
            paths: vec![PathBuf::from("/no/such/tester/path/xyz")],
            last_root: Some(PathBuf::from("/no/such/tester/path/xyz")),
        };
        r.prune_missing();
        assert!(r.paths.is_empty());
        assert!(r.last_root.is_none());
    }

    #[test]
    fn push_sets_last_root() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        fs::create_dir_all(&a).unwrap();
        let mut r = Recents::default();
        r.push(&a);
        assert_eq!(r.last_root.as_ref().unwrap(), &a.canonicalize().unwrap());
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        fs::create_dir_all(&project).unwrap();
        let store = dir.path().join("recents.json");
        let mut r = Recents::default();
        r.push(&project);
        r.save_to(&store).unwrap();
        let loaded = Recents::load_from(&store);
        assert_eq!(loaded.paths.len(), 1);
        assert_eq!(loaded.paths[0], project.canonicalize().unwrap());
    }
}
