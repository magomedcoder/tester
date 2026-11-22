use super::OverlayAction;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::path::Path;
use std::process::Command;
use tester_core::i18n::{self, Msg};

#[derive(Debug)]
pub struct DiffOverlay {
    pub text: String,
    pub scroll: u16,
    pub title: String,
}

impl DiffOverlay {
    pub fn from_git(root: &Path) -> Self {
        let text = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(root)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_else(|_| i18n::t(Msg::DiffUnavailable).into());
        let text = if text.trim().is_empty() {
            i18n::t(Msg::DiffClean).into()
        } else {
            text
        };
        Self {
            text,
            scroll: 0,
            title: i18n::t(Msg::DiffTitle).into(),
        }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let lines: Vec<Line> = self
            .text
            .lines()
            .map(|l| {
                let style = if l.starts_with('+') && !l.starts_with("+++") {
                    theme.diff_added()
                } else if l.starts_with('-') && !l.starts_with("---") {
                    theme.diff_removed()
                } else if l.starts_with("@@") {
                    theme.info_style()
                } else {
                    theme.base()
                };
                Line::from(Span::styled(l.to_string(), style))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n::tr(
                            Msg::DiffOverlayChrome,
                            &[("title", self.title.trim())],
                        ))
                        .border_style(theme.border_block(true))
                        .style(theme.base()),
                ),
            area,
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::PageUp | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(5);
                OverlayAction::None
            }
            KeyCode::PageDown | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(5);
                OverlayAction::None
            }
            _ => OverlayAction::None,
        }
    }
}
