use super::OverlayAction;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use tester_core::i18n::{self, Msg};

#[derive(Debug)]
pub struct ManualOverlay {
    pub text: String,
    pub scroll: u16,
    pub h_scroll: u16,
    pub search: String,
    pub searching: bool,
    pub toc_idx: usize,
}

impl ManualOverlay {
    pub fn new() -> Self {
        Self {
            text: i18n::t(Msg::ManualContent).into(),
            scroll: 0,
            h_scroll: 0,
            search: String::new(),
            searching: false,
            toc_idx: 0,
        }
    }

    fn headings(&self) -> Vec<(usize, String)> {
        self.text
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim_start();
                if t.starts_with("## ") {
                    Some((i, t.trim_start_matches('#').trim().to_string()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(10)])
            .split(area);

        let heads = self.headings();
        let toc_items: Vec<Line> = heads
            .iter()
            .enumerate()
            .map(|(i, (_, t))| {
                let style = if i == self.toc_idx {
                    theme.selected_accent()
                } else {
                    theme.muted()
                };
                Line::from(Span::styled(format!(" {t}"), style))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(toc_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(i18n::t(Msg::ManualTocTitle))
                    .border_style(theme.border_block(true))
                    .style(theme.base()),
            ),
            chunks[0],
        );

        let title = if self.searching {
            i18n::tr(Msg::ManualTitleSearching, &[("q", &self.search)])
        } else {
            i18n::t(Msg::ManualTitleIdle).to_string()
        };
        let body = if self.searching && !self.search.is_empty() {
            let q = self.search.to_lowercase();
            let filtered: String = self
                .text
                .lines()
                .filter(|l| l.to_lowercase().contains(&q))
                .collect::<Vec<_>>()
                .join("\n");
            if filtered.is_empty() {
                i18n::t(Msg::ManualSearchNoMatches).to_string()
            } else {
                filtered
            }
        } else {
            self.text.clone()
        };
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, self.h_scroll))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(theme.border_block(true))
                        .style(theme.base()),
                ),
            chunks[1],
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if alt && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S')) {
            self.searching = !self.searching;
            if !self.searching {
                self.search.clear();
            }
            return OverlayAction::None;
        }
        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.search.clear();
                }
                KeyCode::Backspace => {
                    self.search.pop();
                }
                KeyCode::Char(c) if !c.is_control() => self.search.push(c),
                _ => {}
            }
            return OverlayAction::None;
        }
        let heads = self.headings();
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::Up | KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(1);
                OverlayAction::None
            }
            KeyCode::Down | KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(1);
                OverlayAction::None
            }
            KeyCode::Left => {
                self.h_scroll = self.h_scroll.saturating_sub(4);
                OverlayAction::None
            }
            KeyCode::Right => {
                self.h_scroll = self.h_scroll.saturating_add(4);
                OverlayAction::None
            }
            KeyCode::Tab if !heads.is_empty() => {
                self.toc_idx = (self.toc_idx + 1) % heads.len();
                self.scroll = heads[self.toc_idx].0 as u16;
                OverlayAction::None
            }
            KeyCode::BackTab if !heads.is_empty() => {
                self.toc_idx = if self.toc_idx == 0 {
                    heads.len() - 1
                } else {
                    self.toc_idx - 1
                };
                self.scroll = heads[self.toc_idx].0 as u16;
                OverlayAction::None
            }
            KeyCode::Enter if !heads.is_empty() => {
                self.scroll = heads[self.toc_idx.min(heads.len() - 1)].0 as u16;
                OverlayAction::None
            }
            _ => OverlayAction::None,
        }
    }
}
