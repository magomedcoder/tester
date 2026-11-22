use crate::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render_lite(theme: &Theme, text: &str) -> Vec<Line<'static>> {
    render_inner(theme, text, false)
}

pub fn render_with_tables(theme: &Theme, text: &str) -> Vec<Line<'static>> {
    render_inner(theme, text, true)
}

fn render_inner(theme: &Theme, text: &str, tables: bool) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut table_buf: Vec<String> = Vec::new();
    for raw in text.lines() {
        let trimmed = raw.trim_start();
        if trimmed.starts_with("```") {
            flush_table(theme, &mut table_buf, &mut out);
            in_fence = !in_fence;
            out.push(Line::from(Span::styled(raw.to_string(), theme.faint())));
            continue;
        }
        if in_fence {
            let style = if raw.starts_with('+') {
                theme.diff_added()
            } else if raw.starts_with('-') && !raw.starts_with("---") {
                theme.diff_removed()
            } else {
                Style::default().fg(theme.fg_muted).bg(theme.bg)
            };
            out.push(Line::from(Span::styled(raw.to_string(), style)));
            continue;
        }
        if tables && raw.contains('|') && raw.matches('|').count() >= 2 {
            table_buf.push(raw.to_string());
            continue;
        }
        flush_table(theme, &mut table_buf, &mut out);
        out.push(Line::from(inline_spans(theme, raw)));
    }
    flush_table(theme, &mut table_buf, &mut out);
    if out.is_empty() {
        out.push(Line::from(Span::styled(String::new(), theme.base())));
    }
    out
}

fn flush_table(theme: &Theme, buf: &mut Vec<String>, out: &mut Vec<Line<'static>>) {
    if buf.is_empty() {
        return;
    }
    for (i, row) in buf.iter().enumerate() {
        let cells: Vec<&str> = row
            .split('|')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();
        if cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            continue;
        }
        let joined = cells.join(" │ ");
        let style = if i == 0 {
            theme.base().add_modifier(Modifier::BOLD)
        } else {
            theme.muted()
        };
        out.push(Line::from(Span::styled(format!("  {joined}"), style)));
    }
    buf.clear();
}

fn inline_spans(theme: &Theme, text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut buf = String::new();
    while i < chars.len() {
        if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), theme.base()));
            }
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            spans.push(Span::styled(
                format!("`{code}`"),
                Style::default()
                    .fg(theme.info)
                    .bg(theme.bg_raised)
                    .add_modifier(Modifier::BOLD),
            ));
            continue;
        }
        if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), theme.base()));
            }
            i += 2;
            let mut bold = String::new();
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            spans.push(Span::styled(
                bold,
                theme.base().add_modifier(Modifier::BOLD),
            ));
            continue;
        }
        buf.push(chars[i]);
        i += 1;
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, theme.base()));
    }
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), theme.base()));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn fences_and_inline() {
        let t = Theme::default();
        let lines = render_lite(&t, "see `x` and **y**\n```\n+ok\n-bad\n```");
        assert!(lines.len() >= 4);
    }

    #[test]
    fn tables() {
        let t = Theme::default();
        let lines = render_with_tables(&t, "| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(lines.len() >= 2);
    }
}
