use super::OverlayAction;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone)]
pub enum PlanDecision {
    Approve,
    Reject,
    Edit,
}

#[derive(Debug)]
pub struct PlanOverlay {
    pub title: String,
    pub body: String,
    pub scroll: u16,
}

impl PlanOverlay {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            scroll: 0,
        }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let footer = Line::from(Span::styled(i18n::t(Msg::PlanOverlayFooter), theme.muted()));
        let mut lines = vec![
            Line::from(Span::styled(self.title.clone(), theme.accent_style())),
            Line::from(Span::raw("")),
        ];
        for l in self.body.lines() {
            lines.push(Line::from(Span::styled(l.to_string(), theme.base())));
        }
        lines.push(Line::from(Span::raw("")));
        lines.push(footer);
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n::t(Msg::PlanOverlayTitle))
                        .border_style(theme.border_block(true))
                        .style(theme.raised()),
                ),
            area,
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc => OverlayAction::PlanDone(PlanDecision::Reject),
            KeyCode::Char('a') | KeyCode::Enter => OverlayAction::PlanDone(PlanDecision::Approve),
            KeyCode::Char('r') => OverlayAction::PlanDone(PlanDecision::Reject),
            KeyCode::Char('e') => OverlayAction::PlanDone(PlanDecision::Edit),
            KeyCode::PageUp | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(3);
                OverlayAction::None
            }
            KeyCode::PageDown | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(3);
                OverlayAction::None
            }
            _ => OverlayAction::None,
        }
    }
}
