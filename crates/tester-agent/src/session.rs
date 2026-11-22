use crate::session_store;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tester_providers::{ChatMessage, Usage};

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

pub fn save_session(root: &Path, session: &AgentSession) -> Result<()> {
    let id = match session_store::active_id(root) {
        Some(id) => id,
        None => session_store::open_for_workspace(root)?.0.id,
    };
    session_store::save(&id, session)
}

pub fn load_session(root: &Path) -> Result<AgentSession> {
    Ok(session_store::open_for_workspace(root)?.1)
}

pub fn clear_session(root: &Path) -> Result<()> {
    let id = match session_store::active_id(root) {
        Some(id) => id,
        None => return Ok(()),
    };
    session_store::clear_messages(&id)
}
