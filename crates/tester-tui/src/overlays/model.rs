use super::OverlayAction;
use crate::popup::{Popup, PopupItem, draw_popup};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Clear;
use tester_core::config::LlmConfig;
use tester_core::i18n::{self, Msg};
use tester_llm::list_models;

#[derive(Debug)]
pub struct ModelOverlay {
    pub models: Vec<String>,
    pub selected: usize,
    pub filter: String,
    pub error: Option<String>,
    config: LlmConfig,
}

impl ModelOverlay {
    pub fn fetch(config: &LlmConfig) -> Self {
        match list_models(config) {
            Ok(models) => {
                let models = if models.is_empty() {
                    vec![i18n::t(Msg::ModelOverlayEmpty).into()]
                } else {
                    models
                };
                let selected = config
                    .model
                    .trim()
                    .is_empty()
                    .then_some(0)
                    .or_else(|| models.iter().position(|m| m == &config.model))
                    .unwrap_or(0);
                Self {
                    models,
                    selected,
                    filter: String::new(),
                    error: None,
                    config: config.clone(),
                }
            }
            Err(err) => Self {
                models: vec![i18n::t(Msg::ModelOverlayEmpty).into()],
                selected: 0,
                filter: String::new(),
                error: Some(format!("{err:#}")),
                config: config.clone(),
            },
        }
    }

    fn refresh(&mut self) {
        *self = Self::fetch(&self.config);
    }

    fn filtered(&self) -> Vec<&String> {
        let f = self.filter.to_lowercase();
        self.models
            .iter()
            .filter(|m| f.is_empty() || m.to_lowercase().contains(&f))
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        frame.render_widget(Clear, area);
        let filtered = self.filtered();
        let items: Vec<PopupItem> = filtered
            .iter()
            .map(|m| {
                let desc = if self.error.is_some() {
                    self.error.clone().unwrap_or_default()
                } else if *m == &self.config.model {
                    "current".into()
                } else {
                    String::new()
                };
                PopupItem::new((*m).clone(), desc)
            })
            .collect();
        draw_popup(
            frame,
            area,
            theme,
            &Popup {
                title: i18n::t(Msg::ModelOverlayTitle),
                items: &items,
                selected: self.selected.min(items.len().saturating_sub(1)),
                show_descriptions: self.error.is_some(),
            },
        );
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        let n = self.filtered().len();
        match key.code {
            KeyCode::Esc => OverlayAction::Close,
            KeyCode::Char('r') | KeyCode::Char('R') if self.filter.is_empty() => {
                self.refresh();
                OverlayAction::None
            }
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
                if let Some(m) = filtered.get(self.selected) {
                    let id = (*m).clone();
                    if id == i18n::t(Msg::ModelOverlayEmpty) {
                        return OverlayAction::Close;
                    }
                    OverlayAction::SelectModel(id)
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
