use crate::llm::{ChatMessage, Usage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentSession {
    pub messages: Vec<ChatMessage>,
    pub usage: Usage,
    pub turns: u32,
    pub tool_calls: u32,
}

impl AgentSession {
    pub fn new() -> Self {
        Self::default()
    }
}

fn path(root: &Path) -> PathBuf {
    root.join(".tester").join("session.json")
}

pub fn save_session(root: &Path, session: &AgentSession) -> Result<()> {
    let p = path(root);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, serde_json::to_string_pretty(session)?)
        .with_context(|| format!("запись {}", p.display()))?;
    Ok(())
}

pub fn load_session(root: &Path) -> Result<AgentSession> {
    let p = path(root);
    let text = std::fs::read_to_string(&p).with_context(|| format!("чтение {}", p.display()))?;
    Ok(serde_json::from_str(&text)?)
}

pub fn clear_session(root: &Path) -> Result<()> {
    let p = path(root);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}
