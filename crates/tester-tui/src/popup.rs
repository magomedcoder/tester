use crate::theme::Theme;
use crate::width::{self, pad_to_width, truncate_end};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone)]
pub struct PopupItem {
    pub label: String,
    pub description: String,
}

impl PopupItem {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Popup<'a> {
    pub title: &'a str,
    pub items: &'a [PopupItem],
    pub selected: usize,
    pub show_descriptions: bool,
}

pub fn draw_popup(frame: &mut Frame<'_>, area: Rect, theme: &Theme, popup: &Popup<'_>) {
    if area.width < 6 || area.height < 2 {
        return;
    }
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border).bg(theme.bg_popup))
        .style(theme.popup())
        .title(Span::styled(
            truncate_end(popup.title, area.width.saturating_sub(2) as usize),
            Style::default().fg(theme.fg_muted).bg(theme.bg_popup),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 2 || inner.height < 1 {
        return;
    }

    if popup.items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                truncate_end(i18n::t(Msg::PopupNoMatches), inner.width as usize),
                Style::default().fg(theme.fg_muted).bg(theme.bg_popup),
            )))
            .style(theme.popup()),
            inner,
        );
        return;
    }

    let selected = popup.selected.min(popup.items.len().saturating_sub(1));
    let view_h = inner.height as usize;
    let start = (selected + 1).saturating_sub(view_h);
    let end = (start + view_h).min(popup.items.len());

    let marker_w = 2usize;
    let avail = inner.width as usize;
    let show_desc = popup.show_descriptions && avail >= 28;

    let raw_label_w = popup
        .items
        .iter()
        .map(|i| width::str_width(&i.label))
        .max()
        .unwrap_or(8);
    let label_max = if show_desc {
        avail.saturating_sub(marker_w + 4).clamp(6, 18)
    } else {
        avail.saturating_sub(marker_w).clamp(4, 24)
    };
    let label_w = raw_label_w.min(label_max).max(4.min(label_max));
    let desc_w = if show_desc {
        avail.saturating_sub(marker_w + label_w + 2)
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, it) in popup.items.iter().enumerate().skip(start).take(end - start) {
        let on = i == selected;
        let row_bg = if on {
            theme.bg_popup_sel
        } else {
            theme.bg_popup
        };
        let marker_style = if on {
            Style::default()
                .fg(theme.accent)
                .bg(row_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_faint).bg(row_bg)
        };
        let label_style = if on {
            theme.popup_selected()
        } else {
            Style::default().fg(theme.fg_muted).bg(row_bg)
        };
        let desc_style = if on {
            Style::default().fg(theme.fg_muted).bg(row_bg)
        } else {
            Style::default().fg(theme.fg_faint).bg(row_bg)
        };
        let marker = if on { "› " } else { "  " };
        let label = pad_to_width(&truncate_end(&it.label, label_w), label_w);

        let mut spans = vec![
            Span::styled(marker.to_string(), marker_style),
            Span::styled(label, label_style),
        ];

        let used = marker_w + label_w;
        let rest = avail.saturating_sub(used);
        if show_desc && desc_w > 4 && !it.description.is_empty() {
            spans.push(Span::styled("  ", Style::default().bg(row_bg)));
            let dw = desc_w.saturating_sub(2).min(rest.saturating_sub(2));
            let desc = pad_to_width(&truncate_end(&it.description, dw), dw);
            spans.push(Span::styled(desc, desc_style));
        } else if rest > 0 {
            spans.push(Span::styled(" ".repeat(rest), Style::default().bg(row_bg)));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines).style(theme.popup()), inner);
}

pub fn place_above(parent: Rect, anchor_y: u16, height: u16) -> Rect {
    if parent.width == 0 || parent.height == 0 {
        return Rect::default();
    }
    let max_h = parent.height.saturating_sub(1).max(1);
    let h = height.min(max_h).max(1);
    let min_y = parent.y;
    let max_y = parent.y + parent.height.saturating_sub(h);
    let preferred = anchor_y.saturating_sub(h);
    let y = preferred.clamp(min_y, max_y);
    let mut rect = Rect {
        x: parent.x,
        y,
        width: parent.width,
        height: h,
    };
    let parent_bottom = parent.y.saturating_add(parent.height);
    let parent_right = parent.x.saturating_add(parent.width);
    if rect.y.saturating_add(rect.height) > parent_bottom {
        rect.height = parent_bottom.saturating_sub(rect.y);
    }
    if rect.x.saturating_add(rect.width) > parent_right {
        rect.width = parent_right.saturating_sub(rect.x);
    }
    rect
}

pub fn popup_height(item_count: usize, max: u16) -> u16 {
    let n = (item_count as u16).saturating_add(2);
    let cap = max.max(2);
    n.clamp(2, cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_above_stays_inside_parent() {
        let parent = Rect {
            x: 2,
            y: 1,
            width: 40,
            height: 20,
        };
        let r = place_above(parent, 18, 12);
        assert!(r.x >= parent.x);
        assert!(r.y >= parent.y);
        assert!(r.y + r.height <= parent.y + parent.height);
        assert_eq!(r.width, parent.width);
    }

    #[test]
    fn place_above_tiny_parent() {
        let parent = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 4,
        };
        let r = place_above(parent, 3, 10);
        assert!(r.height <= parent.height);
        assert!(r.y + r.height <= parent.y + parent.height);
        assert!(r.width <= parent.width);
    }
}
