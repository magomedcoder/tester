use crate::session::AgentSession;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub workspace: PathBuf,
    #[serde(default)]
    pub branch: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub title: String,
}

impl SessionMeta {
    pub fn touch(&mut self) {
        self.updated_at = now_secs();
        if self.branch.is_none() {
            self.branch = git_branch(&self.workspace);
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    nanos.hash(&mut h);
    std::process::id().hash(&mut h);
    format!("{nanos:016x}{:016x}", h.finish())
}

fn tester_home() -> PathBuf {
    if let Ok(p) = std::env::var("TESTER_HOME") {
        return PathBuf::from(p);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tester")
}

fn sessions_root() -> PathBuf {
    tester_home().join("sessions")
}

fn session_dir(id: &str) -> PathBuf {
    sessions_root().join(id)
}

fn meta_path(id: &str) -> PathBuf {
    session_dir(id).join("meta.json")
}

fn body_path(id: &str) -> PathBuf {
    session_dir(id).join("session.json")
}

fn workspace_pointer(workspace: &Path) -> PathBuf {
    workspace.join(".tester").join("session_id")
}

fn git_branch(workspace: &Path) -> Option<String> {
    let out = Command::new("git")
        .args([
            "-C",
            &workspace.display().to_string(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s == "HEAD" {
        None
    } else {
        Some(s)
    }
}

fn canonicalize_workspace(workspace: &Path) -> PathBuf {
    workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf())
}

pub fn read_meta(id: &str) -> Result<SessionMeta> {
    let p = meta_path(id);
    let text = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn write_meta(meta: &SessionMeta) -> Result<()> {
    let dir = session_dir(&meta.id);
    fs::create_dir_all(&dir)?;
    let p = meta_path(&meta.id);
    fs::write(&p, serde_json::to_string_pretty(meta)?)
        .with_context(|| format!("write {}", p.display()))?;
    Ok(())
}

fn load_body(id: &str) -> Result<AgentSession> {
    let p = body_path(id);
    let text = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn save_body(id: &str, session: &AgentSession) -> Result<()> {
    let dir = session_dir(id);
    fs::create_dir_all(&dir)?;
    let p = body_path(id);
    fs::write(&p, serde_json::to_string_pretty(session)?)
        .with_context(|| format!("write {}", p.display()))?;
    Ok(())
}

pub fn set_active(workspace: &Path, id: &str) -> Result<()> {
    let p = workspace_pointer(workspace);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&p, id).with_context(|| format!("write {}", p.display()))?;
    Ok(())
}

pub fn active_id(workspace: &Path) -> Option<String> {
    let text = fs::read_to_string(workspace_pointer(workspace)).ok()?;
    let id = text.trim().to_string();
    if id.is_empty() || !meta_path(&id).is_file() {
        None
    } else {
        Some(id)
    }
}

pub fn create(workspace: &Path) -> Result<(SessionMeta, AgentSession)> {
    let workspace = canonicalize_workspace(workspace);
    let id = new_id();
    let now = now_secs();
    let meta = SessionMeta {
        id: id.clone(),
        workspace: workspace.clone(),
        branch: git_branch(&workspace),
        created_at: now,
        updated_at: now,
        title: String::new(),
    };
    write_meta(&meta)?;
    let session = AgentSession::new();
    save_body(&id, &session)?;
    set_active(&workspace, &id)?;
    Ok((meta, session))
}

pub fn save(id: &str, session: &AgentSession) -> Result<()> {
    let mut meta = read_meta(id)?;
    meta.touch();
    if meta.title.is_empty() {
        meta.title = title_from_session(session);
    }
    write_meta(&meta)?;
    save_body(id, session)?;
    set_active(&meta.workspace, id)?;
    Ok(())
}

fn title_from_session(session: &AgentSession) -> String {
    session
        .messages
        .iter()
        .find(|m| m.role == "user")
        .and_then(|m| m.content.as_deref())
        .map(|c| {
            let line = c.lines().next().unwrap_or(c).trim();
            let mut t = line.chars().take(48).collect::<String>();
            if line.chars().count() > 48 {
                t.push('…');
            }
            t
        })
        .unwrap_or_default()
}

pub fn load(id: &str) -> Result<(SessionMeta, AgentSession)> {
    let meta = read_meta(id)?;
    let session = load_body(id).unwrap_or_default();
    Ok((meta, session))
}

pub fn delete(id: &str) -> Result<()> {
    let dir = session_dir(id);
    if dir.is_dir() {
        fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
    }
    Ok(())
}

pub fn resolve_id(prefix: &str) -> Result<String> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        bail!("empty session id");
    }
    if meta_path(prefix).is_file() {
        return Ok(prefix.to_string());
    }
    let mut hits = list_all()?
        .into_iter()
        .filter(|m| m.id.starts_with(prefix))
        .collect::<Vec<_>>();
    match hits.len() {
        0 => bail!("no session matching '{prefix}'"),
        1 => Ok(hits.remove(0).id),
        _ => bail!("ambiguous session id '{prefix}' ({} matches)", hits.len()),
    }
}

pub fn list_all() -> Result<Vec<SessionMeta>> {
    let root = sessions_root();
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if let Ok(meta) = read_meta(&id) {
            out.push(meta);
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
    Ok(out)
}

pub fn list_for_workspace(workspace: &Path) -> Result<Vec<SessionMeta>> {
    let workspace = canonicalize_workspace(workspace);
    Ok(list_all()?
        .into_iter()
        .filter(|m| canonicalize_workspace(&m.workspace) == workspace)
        .collect())
}

pub fn open_for_workspace(workspace: &Path) -> Result<(SessionMeta, AgentSession)> {
    let workspace = canonicalize_workspace(workspace);
    if let Some(id) = active_id(&workspace) {
        let (meta, session) = load(&id)?;
        set_active(&workspace, &id)?;
        return Ok((meta, session));
    }
    if let Some(meta) = list_for_workspace(&workspace)?.into_iter().next() {
        let session = load_body(&meta.id).unwrap_or_default();
        set_active(&workspace, &meta.id)?;
        return Ok((meta, session));
    }
    create(&workspace)
}

pub fn clear_messages(id: &str) -> Result<()> {
    save(id, &AgentSession::new())
}

#[derive(Debug, Clone)]
pub enum PruneSpec {
    Id(String),
    Workspace(PathBuf),
    OlderThanDays(u64),
    All,
}

pub fn prune(spec: PruneSpec) -> Result<usize> {
    let mut n = 0usize;
    match spec {
        PruneSpec::Id(id) => {
            let id = resolve_id(&id)?;
            delete(&id)?;
            n = 1;
        }
        PruneSpec::Workspace(ws) => {
            for meta in list_for_workspace(&ws)? {
                delete(&meta.id)?;
                n += 1;
            }
        }
        PruneSpec::OlderThanDays(days) => {
            let cutoff = now_secs().saturating_sub(days.saturating_mul(86400));
            for meta in list_all()? {
                if meta.updated_at < cutoff {
                    delete(&meta.id)?;
                    n += 1;
                }
            }
        }
        PruneSpec::All => {
            for meta in list_all()? {
                delete(&meta.id)?;
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn create_list_save_roundtrip() {
        let _g = lock_env();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let ws = dir.path().join("proj");
        fs::create_dir_all(&ws).unwrap();

        unsafe { std::env::set_var("TESTER_HOME", &home) };

        let (meta, mut session) = create(&ws).unwrap();
        assert_eq!(active_id(&ws).as_deref(), Some(meta.id.as_str()));
        session
            .messages
            .push(tester_llm::ChatMessage::user("hello world"));
        save(&meta.id, &session).unwrap();

        let listed = list_for_workspace(&ws).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, meta.id);

        let (meta2, session2) = open_for_workspace(&ws).unwrap();
        assert_eq!(meta2.id, meta.id);
        assert_eq!(session2.messages.len(), 1);
        assert!(meta2.title.contains("hello"));

        let (meta3, _) = create(&ws).unwrap();
        assert_ne!(meta3.id, meta.id);
        assert_eq!(list_for_workspace(&ws).unwrap().len(), 2);

        assert_eq!(prune(PruneSpec::Workspace(ws.clone())).unwrap(), 2);
        assert!(list_for_workspace(&ws).unwrap().is_empty());
        unsafe { std::env::remove_var("TESTER_HOME") };
    }
}
