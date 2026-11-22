use super::app::App;
use super::screens::Screen;
use super::theme::{Theme, truncate_middle};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::{Path, PathBuf};
use tester_core::i18n::{self, Msg};

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let theme = app.theme;
    let full = frame.area();
    frame.render_widget(Block::default().style(theme.base()), full);

    let pad_x: u16 = if full.width > 12 { 2 } else { 0 };
    let pad_y: u16 = if full.height > 10 { 1 } else { 0 };
    let area = Rect {
        x: full.x + pad_x,
        y: full.y + pad_y,
        width: full.width.saturating_sub(pad_x.saturating_mul(2)),
        height: full.height.saturating_sub(pad_y.saturating_mul(2)),
    };

    let show_tabs = app.tabs.len() > 1;
    let tabs_h: u16 = if show_tabs { 1 } else { 0 };
    let header_h: u16 = 3;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Length(tabs_h),
            Constraint::Min(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], app, &theme);
    if show_tabs {
        draw_tabs(frame, chunks[1], app, &theme);
    }

    let body = chunks[2];
    match &mut app.screen {
        Screen::Picker(picker) => picker.draw(frame, body, &theme),
        Screen::Session(session) => session.draw(frame, body, &theme),
    }
}

fn draw_tabs(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let mut spans = Vec::new();
    for (i, tab) in app.tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", theme.faint()));
        }
        let mark = if tab.pending { "•" } else { " " };
        let label = format!("{mark}{}", truncate_middle(&tab.title, 16));
        let style = if i == app.tab_idx {
            theme.selected_accent()
        } else {
            theme.muted()
        };
        spans.push(Span::styled(label, style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(theme.base()), area);
}

fn project_path(app: &App) -> Option<PathBuf> {
    match &app.screen {
        Screen::Session(s) => Some(s.root().to_path_buf()),
        Screen::Picker(p) => Some(p.cwd().to_path_buf()),
    }
}

fn display_path(path: &Path) -> String {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(home) = home {
        if let Ok(rest) = path.strip_prefix(&home) {
            if rest.as_os_str().is_empty() {
                return "~".into();
            }
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let version = env!("CARGO_PKG_VERSION");
    let mut top = vec![
        Span::styled("Tester ", theme.brand()),
        Span::styled(format!("v{version}"), theme.muted()),
    ];
    if let Screen::Session(s) = &app.screen {
        top.push(Span::styled("  ", theme.faint()));
        let model = s.model_name().trim();
        if model.is_empty() {
            top.push(Span::styled(
                i18n::t(Msg::ModelNotSetBadge),
                theme.warning_style(),
            ));
        } else {
            top.push(Span::styled(
                truncate_middle(model, 24),
                theme.accent_style(),
            ));
        }
        top.push(Span::styled(" ", theme.faint()));
        top.push(Span::styled(s.conn_glyph().to_string(), theme.info_style()));
    }
    if app.dry_run {
        top.push(Span::styled("", theme.faint()));
        top.push(Span::styled(
            i18n::t(Msg::TuiDryRunBadge),
            theme.warning_style().add_modifier(Modifier::BOLD),
        ));
    }

    let path_raw = project_path(app)
        .map(|p| display_path(&p))
        .unwrap_or_else(|| i18n::t(Msg::TuiNoProject).into());

    let path = truncate_middle(&path_raw, area.width.saturating_sub(2) as usize);
    let bottom = Line::from(Span::styled(path, theme.muted()));

    let widget = Paragraph::new(vec![Line::from(top), bottom]).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme.border_block(false))
            .style(theme.base()),
    );
    frame.render_widget(widget, area);
}
