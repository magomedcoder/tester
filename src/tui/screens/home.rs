use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::PathBuf;

pub struct HomeScreen {
    pub path: Option<PathBuf>,
}

impl HomeScreen {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn handle_key(&mut self, _key: KeyEvent) {}

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        let path = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(no project selected)".into());

        let body = Text::from(vec![
            Line::from("Welcome to Tester TUI"),
            Line::from(""),
            Line::from(format!("Project: {path}")),
            Line::from(""),
            Line::from("Next: project/folder picker"),
            Line::from("Press q or Esc to quit."),
        ]);

        let widget = Paragraph::new(body).block(
            Block::default()
                .title(" home ")
                .borders(Borders::ALL),
        );
        frame.render_widget(widget, area);
    }
}
