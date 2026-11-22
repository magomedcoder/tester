use super::OverlayAction;
use crate::popup::{Popup, PopupItem, draw_popup};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use std::path::PathBuf;
use tester_agent::session_store::{self, SessionMeta};
use tester_core::i18n::{self, Msg};

#[derive(Debug)]
pub struct ChatSessionsOverlay {
    pub items: Vec<SessionMeta>,
    pub selected: usize,
}

impl ChatSessionsOverlay {
    pub fn new(workspace: PathBuf) -> Self {
        let mut items = session_store::list_for_workspace(&workspace).unwrap_or_default();
        if items.is_empty() {
            items = session_store::list_all().unwrap_or_default();
        }
        Self { items, selected: 0 }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let popup_items: Vec<PopupItem> = if self.items.is_empty() {
            vec![PopupItem::new(
                i18n::t(Msg::SessionsNone),
                i18n::t(Msg::SessionsNewHint),
            )]
        } else {
            self.items
                .iter()
                .map(|m| {
                    let id8 = &m.id[..m.id.len().min(8)];
                    let branch = m.branch.as_deref().unwrap_or("-");
                    let title = if m.title.is_empty() {
                        i18n::t(Msg::SessionTitleEmpty)
                    } else {
                        m.title.as_str()
                    };
                    PopupItem::new(
                        format!("{id8}  {branch}  {title}"),
                        m.workspace.display().to_string(),
                    )
                })
                .collect()
        };
        draw_popup(
            frame,
            area,
            theme,
            &Popup {
                title: i18n::t(Msg::SessionsOverlayTitle),
                items: &popup_items,
                selected: self.selected,
                show_descriptions: true,
            },
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::Char('n') | KeyCode::Char('N') => OverlayAction::NewChatSession,
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
                if let Some(m) = self.items.get(self.selected) {
                    OverlayAction::ResumeChatSession(m.id.clone())
                } else {
                    OverlayAction::NewChatSession
                }
            }
            _ => OverlayAction::None,
        }
    }
}
