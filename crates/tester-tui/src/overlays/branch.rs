use super::OverlayAction;
use crate::popup::{Popup, PopupItem, draw_popup};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use std::path::Path;
use std::process::Command;
use tester_core::i18n::{self, Msg};

#[derive(Debug)]
pub struct BranchOverlay {
    pub branches: Vec<String>,
    pub selected: usize,
    pub filter: String,
}

impl BranchOverlay {
    pub fn from_git(root: &Path) -> Self {
        let out = Command::new("git")
            .args(["branch", "--list", "--format=%(refname:short)"])
            .current_dir(root)
            .output()
            .ok();
        let mut branches = out
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if branches.is_empty() {
            branches.push(i18n::t(Msg::BranchNone).into());
        }
        Self {
            branches,
            selected: 0,
            filter: String::new(),
        }
    }

    fn filtered(&self) -> Vec<&String> {
        let f = self.filter.to_lowercase();
        self.branches
            .iter()
            .filter(|b| f.is_empty() || b.to_lowercase().contains(&f))
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let filtered = self.filtered();
        let items: Vec<PopupItem> = filtered
            .iter()
            .map(|b| PopupItem::new((*b).clone(), i18n::t(Msg::BranchItemTag)))
            .collect();
        draw_popup(
            frame,
            area,
            theme,
            &Popup {
                title: i18n::t(Msg::BranchOverlayTitle),
                items: &items,
                selected: self.selected.min(items.len().saturating_sub(1)),
                show_descriptions: false,
            },
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        let n = self.filtered().len();
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                OverlayAction::None
            }
            KeyCode::Down => {
                if n > 0 {
                    self.selected = (self.selected + 1).min(n - 1);
                }
                OverlayAction::None
            }
            KeyCode::Enter => {
                let filtered = self.filtered();
                if let Some(b) = filtered.get(self.selected) {
                    OverlayAction::SwitchBranch((*b).clone())
                } else {
                    OverlayAction::Close
                }
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected = 0;
                OverlayAction::None
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.selected = 0;
                OverlayAction::None
            }
            _ => OverlayAction::None,
        }
    }
}
