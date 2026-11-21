use super::app::App;
use super::screens::Screen;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let mut title_spans = vec![
        Span::styled(
            " tester ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("tui"),
    ];
    if app.dry_run {
        title_spans.push(Span::raw("   dry-run"));
    }
    if app.resume {
        title_spans.push(Span::raw("   resume"));
    }
    let title = Paragraph::new(Line::from(title_spans))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    match &app.screen {
        Screen::Home(home) => home.draw(frame, chunks[1]),
        Screen::Picker(picker) => picker.draw(frame, chunks[1]),
    }

    let footer_text = match &app.screen {
        Screen::Home(_) => " Ctrl+O open project    q/Esc quit ",
        Screen::Picker(_) => {
            " ↑↓  Enter open    Ctrl+Enter/Ctrl+S select    Esc back    type to filter "
        }
    };
    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, chunks[2]);
}
