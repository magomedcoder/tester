use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts_unix: u64,
    pub tool: String,
    pub target: String,
    pub result: String,
}

#[derive(Debug, Default)]
pub struct AuditLog {
    inner: Mutex<Vec<AuditEntry>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, tool: &str, target: &str, result: &str) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Ok(mut g) = self.inner.lock() {
            g.push(AuditEntry {
                ts_unix: ts,
                tool: tool.into(),
                target: target.into(),
                result: result.into(),
            });
        }
    }

    pub fn entries(&self) -> Vec<AuditEntry> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn persist(&self, root: &Path) -> anyhow::Result<PathBuf> {
        let path = root.join(".tester").join("audit.jsonl");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for e in self.entries() {
            out.push_str(&serde_json::to_string(&e)?);
            out.push('\n');
        }
        std::fs::write(&path, out)?;
        Ok(path)
    }

    pub fn summary(&self) -> String {
        let entries = self.entries();
        if entries.is_empty() {
            return "аудит: пусто".into();
        }
        let mut lines = vec![format!("аудит: {} действий", entries.len())];
        for e in entries.iter().rev().take(12) {
            lines.push(format!("- {} {} -> {}", e.tool, e.target, e.result));
        }
        lines.join("\n")
    }
}

impl Clone for AuditLog {
    fn clone(&self) -> Self {
        Self {
            inner: Mutex::new(self.entries()),
        }
    }
}
