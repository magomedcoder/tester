use crate::i18n::{self, Msg};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum HomeAction {
    None,
    OpenPicker,
}

pub struct HomeScreen {
    pub path: Option<PathBuf>,
    pub summary: String,
    pub status: String,
}

impl HomeScreen {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut home = Self {
            path: None,
            summary: String::new(),
            status: i18n::t(Msg::HomeStatusIdle).into(),
        };
        if let Some(p) = path {
            home.set_project(p);
        }
        home
    }

    pub fn set_project(&mut self, path: PathBuf) {
        match crate::discover::discover(&path) {
            Ok(profile) => {
                self.summary = format!(
                    "{}={} {}={} {}={} {}={}",
                    i18n::t(Msg::Language),
                    profile.language.as_str(),
                    i18n::t(Msg::Supported),
                    profile.supported,
                    i18n::t(Msg::Package),
                    profile.package_name,
                    i18n::t(Msg::TestCommand),
                    profile.test_command
                );
                self.path = Some(profile.root);
                self.status = i18n::t(Msg::HomeStatusSelected).into();
            }
            Err(err) => {
                self.path = Some(path);
                self.summary = format!("{}: {err:#}", i18n::t(Msg::DiscoverFailed));
                self.status = i18n::t(Msg::HomeStatusWarn).into();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> HomeAction {
        match key.code {
            KeyCode::Char('o') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                HomeAction::OpenPicker
            }
            KeyCode::Char('o') => HomeAction::OpenPicker,
            _ => HomeAction::None,
        }
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        let path = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| i18n::t(Msg::NoProject).into());

        let body = Text::from(vec![
            Line::from("Tester TUI"),
            Line::from(""),
            Line::from(format!("{}: {path}", i18n::t(Msg::ProjectLabel))),
            Line::from(if self.summary.is_empty() {
                String::new()
            } else {
                self.summary.clone()
            }),
            Line::from(""),
            Line::from(self.status.clone()),
            Line::from(""),
            Line::from(i18n::t(Msg::HomeHint)),
        ]);

        let widget = Paragraph::new(body).block(
            Block::default()
                .title(i18n::t(Msg::TitleHome))
                .borders(Borders::ALL),
        );
        frame.render_widget(widget, area);
    }
}
