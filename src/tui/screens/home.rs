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
            status: "Ctrl+O open project  q quit".into(),
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
                    "language={} supported={} package={} tests={}",
                    profile.language.as_str(),
                    profile.supported,
                    profile.package_name,
                    profile.test_command
                );
                self.path = Some(profile.root);
                self.status = "project selected  Ctrl+O switch  q quit".into();
            }
            Err(err) => {
                self.path = Some(path);
                self.summary = format!("discover failed: {err:#}");
                self.status = "path set with warnings  Ctrl+O switch  q quit".into();
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
            .unwrap_or_else(|| "(no project selected)".into());

        let body = Text::from(vec![
            Line::from("Tester TUI"),
            Line::from(""),
            Line::from(format!("Project: {path}")),
            Line::from(if self.summary.is_empty() {
                String::new()
            } else {
                self.summary.clone()
            }),
            Line::from(""),
            Line::from(self.status.clone()),
            Line::from(""),
            Line::from("Session chat comes next - for now pick a project."),
        ]);

        let widget = Paragraph::new(body).block(
            Block::default()
                .title(" home ")
                .borders(Borders::ALL),
        );
        frame.render_widget(widget, area);
    }
}
