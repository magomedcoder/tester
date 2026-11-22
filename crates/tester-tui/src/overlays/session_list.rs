use super::OverlayAction;
use crate::popup::{Popup, PopupItem, draw_popup};
use crate::recents::Recents;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use std::path::PathBuf;
use tester_core::i18n::{self, Msg};

#[derive(Debug)]
pub struct SessionListOverlay {
    pub items: Vec<PathBuf>,
    pub selected: usize,
}

impl SessionListOverlay {
    pub fn new(current: Option<PathBuf>) -> Self {
        let mut items = Recents::load().paths;
        if let Some(c) = current {
            if !items.iter().any(|p| p == &c) {
                items.insert(0, c);
            }
        }
        Self { items, selected: 0 }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let popup_items: Vec<PopupItem> = self
            .items
            .iter()
            .map(|p| {
                let label = p.display().to_string();
                let desc = if p.join(".tester").is_dir() {
                    i18n::t(Msg::RecentHasTester)
                } else {
                    i18n::t(Msg::RecentProject)
                };
                PopupItem::new(label, desc)
            })
            .collect();
        draw_popup(
            frame,
            area,
            theme,
            &Popup {
                title: i18n::t(Msg::RecentOverlayTitle),
                items: &popup_items,
                selected: self.selected,
                show_descriptions: true,
            },
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                OverlayAction::None
            }
            KeyCode::Down => {
                if !self.items.is_empty() {
                    self.selected = (self.selected + 1).min(self.items.len() - 1);
                }
                OverlayAction::None
            }
            KeyCode::Enter => {
                if let Some(p) = self.items.get(self.selected) {
                    OverlayAction::SwitchSession(p.clone())
                } else {
                    OverlayAction::Close
                }
            }
            _ => OverlayAction::None,
        }
    }
}
