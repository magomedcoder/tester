use crate::adapter;
use crate::i18n::{self, Msg};
use crate::tui::recents::Recents;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub enum PickerAction {
    None,
    Cancel,
    Selected(PathBuf),
}

#[derive(Debug, Clone)]
enum Row {
    Recent(PathBuf),
    Parent,
    Dir(PathBuf, bool),
    Worktree(PathBuf),
}

pub struct PickerScreen {
    cwd: PathBuf,
    filter: String,
    selected: usize,
    rows: Vec<Row>,
    recents: Recents,
    status: String,
    show_recents: bool,
    worktree_mode: bool,
}

impl PickerScreen {
    pub fn new(start: Option<PathBuf>) -> Self {
        let cwd = start
            .and_then(|p| {
                if p.is_file() {
                    p.parent().map(|x| x.to_path_buf())
                } else {
                    Some(p)
                }
            })
            .filter(|p| p.is_dir())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(home_dir);
        let cwd = cwd.canonicalize().unwrap_or(cwd);
        let mut s = Self {
            cwd,
            filter: String::new(),
            selected: 0,
            rows: Vec::new(),
            recents: Recents::load(),
            status: String::new(),
            show_recents: true,
            worktree_mode: false,
        };
        s.refresh_rows();
        s
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerAction {
        match key.code {
            KeyCode::Esc => {
                if self.worktree_mode {
                    self.worktree_mode = false;
                    self.refresh_rows();
                    return PickerAction::None;
                }
                return PickerAction::Cancel;
            }
            KeyCode::Right => {
                self.enter_worktree_mode();
            }
            KeyCode::Left => {
                if self.worktree_mode {
                    self.worktree_mode = false;
                    self.refresh_rows();
                } else {
                    self.go_up();
                }
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected + 1 < self.rows.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => {
                if !self.rows.is_empty() {
                    self.selected = self.rows.len() - 1;
                }
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if !self.rows.is_empty() {
                    self.selected = (self.selected + 10).min(self.rows.len() - 1);
                }
            }
            KeyCode::Backspace => {
                if !self.filter.is_empty() {
                    self.filter.pop();
                    self.refresh_rows();
                } else if self.worktree_mode {
                    self.worktree_mode = false;
                    self.refresh_rows();
                } else {
                    self.go_up();
                }
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    return self.confirm_highlighted_or_cwd();
                }
                return self.activate_selected();
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return self.confirm_highlighted_or_cwd();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !key.modifiers.contains(KeyModifiers::SUPER) =>
            {
                self.filter.push(c);
                self.refresh_rows();
            }
            _ => {}
        }
        PickerAction::None
    }

    fn enter_worktree_mode(&mut self) {
        let trees = list_worktrees(&self.cwd);
        if trees.is_empty() {
            self.status = "no git worktrees".into();
            return;
        }
        self.worktree_mode = true;
        self.filter.clear();
        self.show_recents = false;
        self.refresh_rows();
    }

    fn activate_selected(&mut self) -> PickerAction {
        let Some(row) = self.rows.get(self.selected).cloned() else {
            return self.select_path(self.cwd.clone());
        };
        match row {
            Row::Recent(path) | Row::Worktree(path) => self.select_path(path),
            Row::Parent => {
                self.go_up();
                PickerAction::None
            }
            Row::Dir(path, _) => {
                self.enter_dir(path);
                PickerAction::None
            }
        }
    }

    fn confirm_highlighted_or_cwd(&mut self) -> PickerAction {
        match self.rows.get(self.selected).cloned() {
            Some(Row::Recent(path)) | Some(Row::Dir(path, _)) | Some(Row::Worktree(path)) => {
                self.select_path(path)
            }
            _ => self.select_path(self.cwd.clone()),
        }
    }

    fn select_path(&mut self, path: PathBuf) -> PickerAction {
        let root = adapter::find_project_root(&path).unwrap_or_else(|_| path.clone());
        match crate::discover::discover(&root) {
            Ok(profile) => {
                self.status = format!(
                    "ok: {} ({})",
                    profile.language.as_str(),
                    if profile.supported {
                        i18n::t(Msg::SupportedWord)
                    } else {
                        i18n::t(Msg::UnsupportedWord)
                    }
                );
            }
            Err(err) => {
                self.status = format!("warn: {err:#}");
            }
        }
        self.recents.push(&root);
        let _ = self.recents.save();
        PickerAction::Selected(root)
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.filter.clear();
            self.show_recents = false;
            self.worktree_mode = false;
            self.refresh_rows();
        }
    }

    fn enter_dir(&mut self, path: PathBuf) {
        self.cwd = path;
        self.filter.clear();
        self.show_recents = false;
        self.worktree_mode = false;
        self.refresh_rows();
    }

    fn refresh_rows(&mut self) {
        let filter = self.filter.to_lowercase();
        let mut rows = Vec::new();

        if self.worktree_mode {
            let mut trees = list_worktrees(&self.cwd);
            if !filter.is_empty() {
                trees.retain(|p| {
                    p.to_string_lossy()
                        .to_lowercase()
                        .contains(&filter)
                });
            }
            for p in trees {
                rows.push(Row::Worktree(p));
            }
            self.rows = rows;
            if self.selected >= self.rows.len() {
                self.selected = self.rows.len().saturating_sub(1);
            }
            self.status = i18n::t(Msg::PickerStatusWorktrees).into();
            return;
        }

        if self.show_recents && filter.is_empty() {
            for p in &self.recents.paths {
                rows.push(Row::Recent(p.clone()));
            }
        }

        if self.cwd.parent().is_some() {
            rows.push(Row::Parent);
        }

        let mut dirs = list_dirs(&self.cwd);
        if !filter.is_empty() {
            dirs.retain(|d| {
                d.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&filter))
                    .unwrap_or(false)
            });
        }
        dirs.sort_by(|a, b| {
            let an = a.file_name().unwrap_or_default();
            let bn = b.file_name().unwrap_or_default();
            an.cmp(bn)
        });
        for d in dirs {
            let is_project = adapter::has_manifest(&d);
            rows.push(Row::Dir(d, is_project));
        }

        self.rows = rows;
        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
        self.status = if adapter::has_manifest(&self.cwd) {
            i18n::t(Msg::PickerStatusProject).into()
        } else {
            i18n::t(Msg::PickerStatusNav).into()
        };
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(area);

        let crumb = breadcrumb(&self.cwd);
        let filter_line = if self.filter.is_empty() {
            String::new()
        } else {
            format!("  {}: {}", i18n::t(Msg::PickerFilter), self.filter)
        };
        let title = if self.worktree_mode {
            i18n::t(Msg::TitleWorktrees)
        } else {
            i18n::t(Msg::TitleOpenProject)
        };
        let header = Paragraph::new(Line::from(vec![
            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(crumb),
            Span::raw(filter_line),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let mark = if i == self.selected { ">" } else { " " };
                let (label, style) = match row {
                    Row::Recent(p) => (
                        format!("{mark} * {}", short_path(p)),
                        Style::default().add_modifier(Modifier::ITALIC),
                    ),
                    Row::Parent => (format!("{mark} ../"), Style::default()),
                    Row::Worktree(p) => (
                        format!(
                            "{mark} {}  {}",
                            short_path(p),
                            i18n::t(Msg::PickerWorktreeTag)
                        ),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Row::Dir(p, is_project) => {
                        let name = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("?")
                            .to_string();
                        if *is_project {
                            (
                                format!(
                                    "{mark} {name}/  {}",
                                    i18n::t(Msg::PickerProjectTag)
                                ),
                                Style::default().add_modifier(Modifier::BOLD),
                            )
                        } else {
                            (format!("{mark} {name}/"), Style::default())
                        }
                    }
                };
                ListItem::new(label).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(if self.worktree_mode {
                    i18n::t(Msg::TitleWorktrees)
                } else {
                    i18n::t(Msg::TitleFolders)
                }),
        );
        frame.render_widget(list, chunks[1]);

        let status = Paragraph::new(self.status.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(i18n::t(Msg::TitleStatus)),
        );
        frame.render_widget(status, chunks[2]);
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn list_dirs(cwd: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(cwd) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && name != "dist"
                && name != "__pycache__"
        })
        .collect()
}

pub fn list_worktrees(cwd: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["-C", &cwd.to_string_lossy(), "worktree", "list", "--porcelain"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                out.push(p);
            }
        }
    }
    out
}

pub fn breadcrumb(path: &Path) -> String {
    let home = home_dir();
    let display = if let Ok(rest) = path.strip_prefix(&home) {
        if rest.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~/{}", rest.display())
        }
    } else {
        path.display().to_string()
    };
    display.replace('/', " › ")
}

fn short_path(path: &Path) -> String {
    let home = home_dir();
    if let Ok(rest) = path.strip_prefix(&home) {
        format!("~/{}", rest.display())
    } else {
        path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumb_uses_tilde() {
        let home = home_dir();
        let p = home.join("projects").join("demo");
        let b = breadcrumb(&p);
        assert!(b.starts_with('~'), "{b}");
        assert!(b.contains("projects"));
    }

    #[test]
    fn list_dirs_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("visible")).unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        std::fs::create_dir(dir.path().join("node_modules")).unwrap();
        let dirs = list_dirs(dir.path());
        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with("visible"));
    }

    #[test]
    fn list_worktrees_empty_outside_git() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_worktrees(dir.path()).is_empty());
    }
}
