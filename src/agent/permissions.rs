use crate::agent::tools::ToolKind;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    AutoYes,
    Prompt,
    DryRun,
}

#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    pub mode: PermissionMode,
}

impl PermissionPolicy {
    pub fn from_flags(yes: bool, dry_run: bool) -> Self {
        let mode = if dry_run {
            PermissionMode::DryRun
        } else if yes {
            PermissionMode::AutoYes
        } else {
            PermissionMode::Prompt
        };
        Self { mode }
    }

    pub fn allow(&self, kind: ToolKind, tool: &str, preview: &str) -> bool {
        match (self.mode, kind) {
            (_, ToolKind::ReadOnly) => true,
            (PermissionMode::AutoYes, _) => true,
            (PermissionMode::DryRun, _) => true,
            (PermissionMode::Prompt, ToolKind::Write | ToolKind::Shell) => confirm(tool, preview),
        }
    }
}

fn confirm(tool: &str, preview: &str) -> bool {
    let preview: String = preview.chars().take(200).collect();
    eprint!("🔒 Разрешить {tool}? {preview} [y/N] ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim(), "y" | "Y" | "yes" | "д" | "Д" | "да")
}
