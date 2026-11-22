use crate::markdown;
use crate::theme::Theme;
use crate::width::{self, wrap_display};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Plain,
    UserInput,
    Table,
    CodeBlock,
    Collapsible,
    ToolCall,
}

#[derive(Debug, Clone)]
pub struct TranscriptEntry {
    pub role: &'static str,
    pub text: String,
    pub kind: LineKind,
    pub collapsed: bool,
    pub id: u64,
}

impl TranscriptEntry {
    pub fn new(role: &'static str, text: impl Into<String>) -> Self {
        let text = text.into();
        let kind = classify(role, &text);
        let collapsed = matches!(kind, LineKind::Collapsible | LineKind::ToolCall);
        Self {
            role,
            text,
            kind,
            collapsed,
            id: 0,
        }
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }
}

fn classify(role: &str, text: &str) -> LineKind {
    if role == "you" || role == "user" {
        return LineKind::UserInput;
    }
    if role == "tool" || text.lines().any(|l| l.trim_start().starts_with('→')) {
        return LineKind::ToolCall;
    }
    if text.contains("```") {
        return LineKind::CodeBlock;
    }
    if text
        .lines()
        .any(|l| l.contains('|') && l.matches('|').count() >= 2)
    {
        return LineKind::Table;
    }
    if text.chars().count() > 4000 || text.lines().count() > 40 {
        return LineKind::Collapsible;
    }
    LineKind::Plain
}

#[derive(Debug, Default)]
pub struct TranscriptView {
    pub entries: Vec<TranscriptEntry>,
    pub offset: u16,
    pub stick_bottom: bool,
    pub next_id: u64,
    cache_key: (usize, u16, u64, u64),
    cached_lines: Vec<Line<'static>>,
}

impl TranscriptView {
    pub fn new() -> Self {
        Self {
            stick_bottom: true,
            ..Default::default()
        }
    }

    pub fn push(&mut self, role: &'static str, text: impl Into<String>) {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.entries
            .push(TranscriptEntry::new(role, text).with_id(id));
        self.invalidate();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.offset = 0;
        self.stick_bottom = true;
        self.invalidate();
    }

    pub fn invalidate(&mut self) {
        self.cache_key = (0, 0, 0, 0);
        self.cached_lines.clear();
    }

    pub fn scroll_up(&mut self, n: u16) {
        self.stick_bottom = false;
        self.offset = self.offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: u16) {
        self.offset = self.offset.saturating_add(n);
    }

    fn collapse_sig(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| e.collapsed)
            .map(|e| e.id)
            .fold(0u64, |a, b| a ^ b.wrapping_mul(0x9e37))
    }

    fn build_lines(
        &self,
        theme: &Theme,
        width: u16,
        streaming_idx: Option<usize>,
    ) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        if self.entries.is_empty() {
            out.push(Line::from(Span::styled(
                i18n::t(Msg::EmptyTranscript),
                theme.muted(),
            )));
            return out;
        }
        let content_w = width.saturating_sub(2).max(8) as usize;

        for (i, entry) in self.entries.iter().enumerate() {
            let streaming = streaming_idx == Some(i);
            if matches!(
                entry.kind,
                LineKind::Collapsible | LineKind::ToolCall | LineKind::CodeBlock
            ) {
                let badge = if entry.collapsed { "..." } else { "▼" };
                out.push(Line::from(Span::styled(
                    format!("  {badge}"),
                    theme.faint(),
                )));
            }

            let body_text = if entry.collapsed
                && matches!(
                    entry.kind,
                    LineKind::Collapsible | LineKind::ToolCall | LineKind::CodeBlock
                ) {
                let first = entry.text.lines().next().unwrap_or("");
                let n = entry.text.lines().count();
                format!("{first}  ... ({n} lines)")
            } else {
                entry.text.clone()
            };

            let rendered = match entry.kind {
                LineKind::ToolCall => tool_lines(theme, &body_text),
                LineKind::Table => markdown::render_with_tables(theme, &body_text),
                LineKind::CodeBlock
                | LineKind::Plain
                | LineKind::UserInput
                | LineKind::Collapsible => markdown::render_lite(theme, &body_text),
            };

            let body_style = match entry.role {
                "you" | "user" => theme.role_style("you").remove_modifier(Modifier::BOLD),
                "assistant" | "tester" => theme.base(),
                _ => theme.muted(),
            };
            let prefix = match entry.role {
                "you" | "user" => "› ",
                _ => "  ",
            };
            let prefix_w = width::str_width(prefix);
            let wrap_w = content_w.saturating_sub(prefix_w).max(4);

            let last = rendered.len().saturating_sub(1);
            for (li, line) in rendered.into_iter().enumerate() {
                let flat: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                let chunks = wrap_display(&flat, wrap_w);
                let nchunks = chunks.len();
                for (ci, chunk) in chunks.into_iter().enumerate() {
                    let mut spans = vec![Span::styled(prefix.to_string(), body_style)];
                    spans.push(Span::styled(chunk, body_style));
                    if streaming && li == last && ci + 1 == nchunks {
                        spans.push(Span::styled(" ▋", theme.accent_style()));
                    }
                    out.push(Line::from(spans));
                }
            }
            out.push(Line::from(Span::raw("")));
        }
        out
    }

    fn lines_cached(
        &mut self,
        theme: &Theme,
        width: u16,
        streaming_idx: Option<usize>,
    ) -> &[Line<'static>] {
        let sig = self.collapse_sig()
            ^ streaming_idx.unwrap_or(usize::MAX) as u64
            ^ (self.entries.last().map(|e| e.text.len()).unwrap_or(0) as u64);
        let key = (self.entries.len(), width, sig, self.next_id);
        if self.cache_key != key || self.cached_lines.is_empty() {
            self.cached_lines = self.build_lines(theme, width, streaming_idx);
            self.cache_key = key;
        }
        &self.cached_lines
    }

    fn clamp_offset(&mut self, total: u16, view_h: u16) {
        let max = total.saturating_sub(view_h);
        if self.stick_bottom {
            self.offset = max;
        } else {
            self.offset = self.offset.min(max);
            if self.offset >= max {
                self.stick_bottom = true;
            }
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        theme: &Theme,
        streaming_idx: Option<usize>,
    ) {
        if area.width < 4 || area.height < 1 {
            return;
        }
        let lines = self.lines_cached(theme, area.width, streaming_idx).to_vec();
        let total = lines.len() as u16;
        let view_h = area.height.max(1);
        self.clamp_offset(total, view_h);

        let start = self.offset as usize;
        let end = (start + view_h as usize).min(lines.len());
        let visible = if start < lines.len() {
            lines[start..end].to_vec()
        } else {
            Vec::new()
        };
        frame.render_widget(Paragraph::new(visible).style(theme.base()), area);

        if total > view_h && area.width > 2 {
            let track = view_h as usize;
            let thumb_h = ((view_h as f32 / total as f32) * track as f32)
                .ceil()
                .clamp(1.0, track as f32) as usize;
            let max_off = total.saturating_sub(view_h);
            let pos = if max_off == 0 {
                0
            } else {
                ((self.offset as f32 / max_off as f32) * (track.saturating_sub(thumb_h)) as f32)
                    .round() as usize
            };
            let sb_x = area.x + area.width - 1;
            for row in 0..track {
                let on = row >= pos && row < pos + thumb_h;
                let ch = if on { '▐' } else { '│' };
                let style = if on {
                    theme.accent_style()
                } else {
                    theme.faint()
                };
                frame.render_widget(
                    Paragraph::new(Span::styled(ch.to_string(), style)),
                    Rect {
                        x: sb_x,
                        y: area.y + row as u16,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }
}

fn tool_lines(theme: &Theme, text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|l| {
            Line::from(Span::styled(
                format!("  {l}"),
                if l.trim_start().starts_with('+') {
                    theme.diff_added()
                } else if l.trim_start().starts_with('-') {
                    theme.diff_removed()
                } else {
                    theme.muted()
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_tool_and_user() {
        let u = TranscriptEntry::new("you", "hi");
        assert_eq!(u.kind, LineKind::UserInput);
        let t = TranscriptEntry::new("assistant", "→ grep(x)");
        assert_eq!(t.kind, LineKind::ToolCall);
    }

    #[test]
    fn stick_bottom_clamps_to_end() {
        let mut v = TranscriptView::new();
        for i in 0..30 {
            v.push("assistant", format!("line {i}"));
        }
        let theme = Theme::default();
        let lines = v.build_lines(&theme, 40, None);
        let total = lines.len() as u16;
        v.stick_bottom = true;
        v.clamp_offset(total, 10);
        assert_eq!(v.offset, total.saturating_sub(10));
    }

    #[test]
    fn scroll_up_leaves_bottom() {
        let mut v = TranscriptView::new();
        v.offset = 20;
        v.stick_bottom = true;
        v.scroll_up(3);
        assert!(!v.stick_bottom);
        assert_eq!(v.offset, 17);
    }
}
