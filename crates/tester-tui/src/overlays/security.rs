use super::OverlayAction;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::path::Path;
use tester_core::i18n::{self, Msg};
use tester_lang::discover::discover;
use tester_security::{render_markdown, scan_with_config};

#[derive(Debug)]
pub struct SecurityOverlay {
    pub text: String,
    pub scroll: u16,
    pub title: String,
}

impl SecurityOverlay {
    pub fn scan(root: &Path, deep: bool) -> Self {
        let text = match (discover(root), tester_core::config::Config::load(root)) {
            (Ok(profile), Ok(mut cfg)) => {
                if deep {
                    cfg.security.deep = true;
                }
                match scan_with_config(&profile, &cfg.security, Some(cfg.security.deep)) {
                    Ok(report) => render_markdown(&report),
                    Err(e) => i18n::tr(Msg::SecScanFailed, &[("err", &format!("{e:#}"))]),
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                i18n::tr(Msg::SecScanFailed, &[("err", &format!("{e:#}"))])
            }
        };
        Self {
            text,
            scroll: 0,
            title: i18n::t(Msg::SecurityOverlayTitle).into(),
        }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let lines: Vec<Line> = self
            .text
            .lines()
            .map(|l| {
                let style = if l.contains("critical") {
                    theme.error_style()
                } else if l.contains("high") {
                    theme.warning_style()
                } else if l.starts_with('#') {
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
                        .title(format!(" {} Esc/q ", self.title.trim()))
                        .border_style(theme.border_block(true))
                        .style(theme.base()),
                ),
            area,
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Close,
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
