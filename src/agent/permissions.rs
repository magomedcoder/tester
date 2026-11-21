use crate::agent::tools::ToolKind;
use std::io::{self, Write};
use std::sync::Arc;

pub type ConfirmFn = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    AutoYes,
    Prompt,
    DryRun,
    DenyWrites,
}

#[derive(Clone)]
pub struct PermissionPolicy {
    pub mode: PermissionMode,
    confirm_fn: Option<ConfirmFn>,
}

impl std::fmt::Debug for PermissionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermissionPolicy")
            .field("mode", &self.mode)
            .field("confirm_fn", &self.confirm_fn.as_ref().map(|_| "Some(..)"))
            .finish()
    }
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
        Self {
            mode,
            confirm_fn: None,
        }
    }

    pub fn for_tui(yes: bool, dry_run: bool) -> Self {
        let mode = if dry_run {
            PermissionMode::DryRun
        } else if yes {
            PermissionMode::AutoYes
        } else {
            PermissionMode::DenyWrites
        };
        Self {
            mode,
            confirm_fn: None,
        }
    }

    pub fn with_confirm(mut self, confirm: ConfirmFn) -> Self {
        self.mode = PermissionMode::Prompt;
        self.confirm_fn = Some(confirm);
        self
    }

    pub fn allow(&self, kind: ToolKind, tool: &str, preview: &str) -> bool {
        match (self.mode, kind) {
            (_, ToolKind::ReadOnly) => true,
            (PermissionMode::AutoYes, _) => true,
            (PermissionMode::DryRun, _) => true,
            (PermissionMode::DenyWrites, ToolKind::Write | ToolKind::Shell) => {
                if let Some(f) = &self.confirm_fn {
                    f(tool, preview)
                } else {
                    false
                }
            }
            (PermissionMode::Prompt, ToolKind::Write | ToolKind::Shell) => {
                if let Some(f) = &self.confirm_fn {
                    f(tool, preview)
                } else {
                    confirm(tool, preview)
                }
            }
        }
    }
}

fn confirm(tool: &str, preview: &str) -> bool {
    let preview: String = preview.chars().take(200).collect();
    eprint!("Allow {tool}? {preview} [y/N] ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim(), "y" | "Y" | "yes" | "д" | "Д" | "да")
}
