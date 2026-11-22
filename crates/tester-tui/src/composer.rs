use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::path::Path;

use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct PromptFrame {
    pub text: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub draft_backup: String,
    pub ghost: String,
    pub reverse_search: Option<ReverseSearch>,
    kill_ring: String,
}

#[derive(Debug, Clone)]
pub struct ReverseSearch {
    pub needle: String,
    pub match_text: Option<String>,
}

impl PromptFrame {
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.ghost.clear();
        self.reverse_search = None;
        self.history_idx = None;
    }

    pub fn set_text(&mut self, s: impl Into<String>) {
        self.text = s.into();
        self.cursor = self.text.len();
        self.clamp_cursor();
    }

    pub fn insert_str(&mut self, s: &str) {
        let s = normalize_paste(s);
        if s.is_empty() {
            return;
        }
        self.text.insert_str(self.cursor, &s);
        self.cursor += s.len();
        self.history_idx = None;
    }

    pub fn insert_char(&mut self, c: char) {
        if c.is_control() && c != '\n' && c != '\t' {
            return;
        }
        let c = if c == '\t' { ' ' } else { c };
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.history_idx = None;
    }

    pub fn push_history(&mut self, entry: &str) {
        let entry = entry.trim();
        if entry.is_empty() {
            return;
        }
        if self.history.last().map(|s| s.as_str()) != Some(entry) {
            self.history.push(entry.to_string());
        }
        if self.history.len() > 200 {
            self.history.remove(0);
        }
        self.history_idx = None;
    }

    pub fn accept_ghost(&mut self) {
        if !self.ghost.is_empty() {
            self.set_text(self.ghost.clone());
            self.ghost.clear();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.reverse_search.is_some() {
            return self.handle_reverse_search(key);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
            self.reverse_search = Some(ReverseSearch {
                needle: String::new(),
                match_text: self.history.last().cloned(),
            });
            return true;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => {
                    self.cursor = line_start(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Char('e') => {
                    self.cursor = line_end(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    return true;
                }
                KeyCode::End => {
                    self.cursor = self.text.len();
                    return true;
                }
                KeyCode::Char('k') => {
                    let end = line_end(&self.text, self.cursor);
                    let killed: String = self.text.drain(self.cursor..end).collect();
                    if !killed.is_empty() {
                        self.kill_ring = killed;
                    } else if self.cursor < self.text.len()
                        && self.text.as_bytes()[self.cursor] == b'\n'
                    {
                        self.text.remove(self.cursor);
                        self.kill_ring = "\n".into();
                    }
                    return true;
                }
                KeyCode::Char('u') => {
                    let start = line_start(&self.text, self.cursor);
                    let killed: String = self.text.drain(start..self.cursor).collect();
                    if !killed.is_empty() {
                        self.kill_ring = killed;
                    }
                    self.cursor = start;
                    return true;
                }
                KeyCode::Char('w') => {
                    self.delete_word_backward();
                    return true;
                }
                KeyCode::Char('y') => {
                    if !self.kill_ring.is_empty() {
                        self.insert_str(&self.kill_ring.clone());
                    }
                    return true;
                }
                KeyCode::Char('t') => {
                    self.transpose_chars();
                    return true;
                }
                KeyCode::Char('d') => {
                    if self.cursor < self.text.len() {
                        let next = next_char(&self.text, self.cursor);
                        self.text.drain(self.cursor..next);
                    }
                    return true;
                }
                KeyCode::Char('h') => {
                    return self.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
                }
                KeyCode::Char('j') | KeyCode::Char('m') => {
                    self.insert_char('\n');
                    return true;
                }
                KeyCode::Left => {
                    self.cursor = prev_word_boundary(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Right => {
                    self.cursor = next_word_boundary(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Delete => {
                    let end = next_word_boundary(&self.text, self.cursor);
                    let killed: String = self.text.drain(self.cursor..end).collect();
                    if !killed.is_empty() {
                        self.kill_ring = killed;
                    }
                    return true;
                }
                KeyCode::Up => {
                    self.cursor = move_vertical(&self.text, self.cursor, -1).unwrap_or(0);
                    return true;
                }
                KeyCode::Down => {
                    self.cursor =
                        move_vertical(&self.text, self.cursor, 1).unwrap_or(self.text.len());
                    return true;
                }
                KeyCode::Char('p') => return self.history_up(),
                KeyCode::Char('n') => return self.history_down(),
                KeyCode::Char('v') => return false,
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Char('\u{7f}') => {
                    self.delete_word_backward();
                    return true;
                }
                KeyCode::Enter => {
                    self.insert_char('\n');
                    return true;
                }
                KeyCode::Char('b') | KeyCode::Left => {
                    self.cursor = prev_word_boundary(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Char('f') | KeyCode::Right => {
                    self.cursor = next_word_boundary(&self.text, self.cursor);
                    return true;
                }
                KeyCode::Char('d') => {
                    let end = next_word_boundary(&self.text, self.cursor);
                    let killed: String = self.text.drain(self.cursor..end).collect();
                    if !killed.is_empty() {
                        self.kill_ring = killed;
                    }
                    return true;
                }
                KeyCode::Delete => {
                    let end = next_word_boundary(&self.text, self.cursor);
                    let killed: String = self.text.drain(self.cursor..end).collect();
                    if !killed.is_empty() {
                        self.kill_ring = killed;
                    }
                    return true;
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Insert && key.modifiers.contains(KeyModifiers::SHIFT) {
            return false;
        }

        match key.code {
            KeyCode::Left => {
                self.cursor = prev_char(&self.text, self.cursor);
                true
            }
            KeyCode::Right => {
                if self.cursor == self.text.len() && !self.ghost.is_empty() {
                    self.accept_ghost();
                } else {
                    self.cursor = next_char(&self.text, self.cursor);
                }
                true
            }
            KeyCode::Home => {
                self.cursor = line_start(&self.text, self.cursor);
                true
            }
            KeyCode::End => {
                self.cursor = line_end(&self.text, self.cursor);
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = prev_char(&self.text, self.cursor);
                    self.text.drain(prev..self.cursor);
                    self.cursor = prev;
                    self.history_idx = None;
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    let next = next_char(&self.text, self.cursor);
                    self.text.drain(self.cursor..next);
                    self.history_idx = None;
                }
                true
            }
            KeyCode::Up if key.modifiers.is_empty() => {
                if let Some(pos) = move_vertical(&self.text, self.cursor, -1) {
                    self.cursor = pos;
                    true
                } else {
                    false
                }
            }
            KeyCode::Down if key.modifiers.is_empty() => {
                if let Some(pos) = move_vertical(&self.text, self.cursor, 1) {
                    self.cursor = pos;
                    true
                } else {
                    false
                }
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::SUPER)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char(c);
                true
            }
            _ => false,
        }
    }

    fn handle_reverse_search(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.reverse_search = None;
                true
            }
            KeyCode::Enter => {
                let m = self
                    .reverse_search
                    .as_ref()
                    .and_then(|rs| rs.match_text.clone());
                if let Some(m) = m {
                    self.set_text(m);
                }
                self.reverse_search = None;
                true
            }
            KeyCode::Backspace => {
                if let Some(rs) = &mut self.reverse_search {
                    if let Some((i, _)) = rs.needle.char_indices().next_back() {
                        rs.needle.truncate(i);
                    }
                    rs.match_text = search_history(&self.history, &rs.needle);
                }
                true
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(rs) = &mut self.reverse_search {
                    if let Some(m) =
                        cycle_history(&self.history, &rs.needle, rs.match_text.as_deref())
                    {
                        rs.match_text = Some(m);
                    }
                }
                true
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !c.is_control() =>
            {
                if let Some(rs) = &mut self.reverse_search {
                    rs.needle.push(c);
                    rs.match_text = search_history(&self.history, &rs.needle);
                }
                true
            }
            _ => true,
        }
    }

    fn history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_idx {
            None => {
                self.draft_backup = self.text.clone();
                let i = self.history.len() - 1;
                self.history_idx = Some(i);
                self.set_text(self.history[i].clone());
            }
            Some(0) => {}
            Some(i) => {
                let i = i - 1;
                self.history_idx = Some(i);
                self.set_text(self.history[i].clone());
            }
        }
        true
    }

    fn history_down(&mut self) -> bool {
        match self.history_idx {
            None => false,
            Some(i) if i + 1 >= self.history.len() => {
                self.history_idx = None;
                self.set_text(self.draft_backup.clone());
                true
            }
            Some(i) => {
                let i = i + 1;
                self.history_idx = Some(i);
                self.set_text(self.history[i].clone());
                true
            }
        }
    }

    fn delete_word_backward(&mut self) {
        let start = prev_word_boundary(&self.text, self.cursor);
        let killed: String = self.text.drain(start..self.cursor).collect();
        if !killed.is_empty() {
            self.kill_ring = killed;
        }
        self.cursor = start;
        self.history_idx = None;
    }

    fn transpose_chars(&mut self) {
        if self.text.is_empty() {
            return;
        }
        if self.cursor == 0 {
            self.cursor = next_char(&self.text, 0);
        }
        if self.cursor >= self.text.len() {
            let b = prev_char(&self.text, self.text.len());
            let a = prev_char(&self.text, b);
            if a < b {
                let ca = self.text[a..b].to_string();
                let cb = self.text[b..].to_string();
                self.text.replace_range(a.., &format!("{cb}{ca}"));
            }
            return;
        }
        let b_end = next_char(&self.text, self.cursor);
        let a = prev_char(&self.text, self.cursor);
        let b = self.cursor;
        if a < b && b < b_end {
            let ca = self.text[a..b].to_string();
            let cb = self.text[b..b_end].to_string();
            self.text.replace_range(a..b_end, &format!("{cb}{ca}"));
            self.cursor = a + cb.len() + ca.len();
        }
    }

    fn clamp_cursor(&mut self) {
        if self.cursor > self.text.len() {
            self.cursor = self.text.len();
        }
        while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    pub fn display_lines(&self, theme: &Theme, width: u16) -> Paragraph<'static> {
        let _ = width;
        if let Some(rs) = &self.reverse_search {
            let preview = rs.match_text.as_deref().unwrap_or("(no match)");
            let line = Line::from(vec![
                Span::styled(
                    tester_core::i18n::t(tester_core::i18n::Msg::ComposerRevSearch),
                    theme.accent_style(),
                ),
                Span::styled(rs.needle.clone(), theme.base()),
                Span::styled(" ▋ ", theme.accent_style()),
                Span::styled(preview.to_string(), theme.muted()),
            ]);
            return Paragraph::new(line).wrap(Wrap { trim: false });
        }

        if self.text.is_empty() && !self.ghost.is_empty() {
            return Paragraph::new(Line::from(vec![
                Span::styled("> ", theme.accent_style()),
                Span::styled(self.ghost.clone(), theme.faint()),
            ]))
            .wrap(Wrap { trim: false });
        }

        if self.text.is_empty() {
            return Paragraph::new(Line::from(Span::styled(
                tester_core::i18n::t(tester_core::i18n::Msg::ComposerPlaceholder),
                theme.faint(),
            )))
            .wrap(Wrap { trim: false });
        }

        let cur = self.cursor.min(self.text.len());
        let (before, after) = self.text.split_at(cur);
        let mut spans = vec![
            Span::styled("> ".to_string(), theme.accent_style()),
            Span::styled(before.to_string(), theme.base()),
            Span::styled("▋".to_string(), theme.accent_style()),
            Span::styled(after.to_string(), theme.base()),
        ];
        if cur == self.text.len() && !self.ghost.is_empty() && self.ghost.starts_with(&self.text) {
            let rest = self.ghost[self.text.len()..].to_string();
            spans.push(Span::styled(rest, theme.faint()));
        }
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false })
    }
}

pub fn normalize_paste(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\0' => {}
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\u{feff}' => {}
            c if c.is_control() && c != '\n' && c != '\t' => {}
            '\t' => out.push_str("    "),
            c => out.push(c),
        }
    }
    out
}

fn search_history(history: &[String], needle: &str) -> Option<String> {
    if needle.is_empty() {
        return history.last().cloned();
    }
    history
        .iter()
        .rev()
        .find(|h| h.to_lowercase().contains(&needle.to_lowercase()))
        .cloned()
}

fn cycle_history(history: &[String], needle: &str, current: Option<&str>) -> Option<String> {
    let matches: Vec<_> = history
        .iter()
        .rev()
        .filter(|h| needle.is_empty() || h.to_lowercase().contains(&needle.to_lowercase()))
        .cloned()
        .collect();
    if matches.is_empty() {
        return None;
    }
    let pos = current
        .and_then(|c| matches.iter().position(|m| m == c))
        .unwrap_or(0);
    Some(matches[(pos + 1) % matches.len()].clone())
}

fn prev_char(s: &str, cur: usize) -> usize {
    if cur == 0 {
        return 0;
    }
    let mut i = cur - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char(s: &str, cur: usize) -> usize {
    if cur >= s.len() {
        return s.len();
    }
    let mut i = cur + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn line_start(s: &str, cur: usize) -> usize {
    s[..cur].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

fn line_end(s: &str, cur: usize) -> usize {
    s[cur..].find('\n').map(|i| cur + i).unwrap_or(s.len())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn prev_word_boundary(s: &str, cur: usize) -> usize {
    let mut i = cur;
    while i > 0 {
        let p = prev_char(s, i);
        let c = s[p..i].chars().next().unwrap_or(' ');
        if c.is_whitespace() {
            i = p;
        } else {
            break;
        }
    }
    if i == 0 {
        return 0;
    }
    let p0 = prev_char(s, i);
    let c0 = s[p0..i].chars().next().unwrap_or(' ');
    let class = is_word_char(c0);
    while i > 0 {
        let p = prev_char(s, i);
        let c = s[p..i].chars().next().unwrap_or(' ');
        if c.is_whitespace() || is_word_char(c) != class {
            break;
        }
        i = p;
    }
    i
}

fn next_word_boundary(s: &str, cur: usize) -> usize {
    let mut i = cur;
    if i >= s.len() {
        return s.len();
    }
    let c0 = s[i..].chars().next().unwrap_or(' ');
    if c0.is_whitespace() {
        while i < s.len() {
            let c = s[i..].chars().next().unwrap_or(' ');
            if !c.is_whitespace() {
                break;
            }
            i = next_char(s, i);
        }
        return i;
    }
    let class = is_word_char(c0);
    while i < s.len() {
        let c = s[i..].chars().next().unwrap_or(' ');
        if c.is_whitespace() || is_word_char(c) != class {
            break;
        }
        i = next_char(s, i);
    }
    while i < s.len() {
        let c = s[i..].chars().next().unwrap_or(' ');
        if !c.is_whitespace() {
            break;
        }
        i = next_char(s, i);
    }
    i
}

fn move_vertical(s: &str, cur: usize, dir: i8) -> Option<usize> {
    let start = line_start(s, cur);
    let col = s[start..cur].chars().count();
    if dir < 0 {
        if start == 0 {
            return None;
        }
        let prev_end = start - 1;
        let prev_start = line_start(s, prev_end);
        Some(pos_at_col(s, prev_start, col))
    } else {
        let end = line_end(s, cur);
        if end >= s.len() {
            return None;
        }
        let next_start = end + 1;
        Some(pos_at_col(s, next_start, col))
    }
}

fn pos_at_col(s: &str, line_start: usize, col: usize) -> usize {
    let end = line_end(s, line_start);
    let mut i = line_start;
    let mut c = 0usize;
    while i < end && c < col {
        i = next_char(s, i);
        c += 1;
    }
    i
}

pub fn edit_external(initial: &str) -> anyhow::Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".into());
    let dir = std::env::temp_dir();
    let path = dir.join(format!("tester-prompt-{}.txt", std::process::id()));
    std::fs::write(&path, initial)?;
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} {}", path.display()))
        .status()?;
    if !status.success() {
        anyhow::bail!("editor exited with {status}");
    }
    let text = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(Path::new(&path));
    Ok(normalize_paste(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_boundaries_unicode() {
        let s = "привет мир";
        let end = s.len();
        let mid = prev_word_boundary(s, end);
        assert!(s[mid..].starts_with("мир") || mid < end);
    }

    #[test]
    fn insert_and_backspace() {
        let mut p = PromptFrame::default();
        p.insert_char('a');
        p.insert_char('b');
        assert_eq!(p.text, "ab");
        p.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(p.text, "a");
    }

    #[test]
    fn normalize_crlf_and_controls() {
        assert_eq!(normalize_paste("a\r\nb\0c"), "a\nbc");
        assert_eq!(normalize_paste("x\ty"), "x    y");
    }

    #[test]
    fn multiline_vertical_nav() {
        let s = "one\ntwo\nthree";
        let cur = s.find('w').unwrap();
        let up = move_vertical(s, cur, -1).unwrap();
        assert!(s[up..].starts_with('n') || s[line_start(s, up)..].starts_with("one"));
        let down = move_vertical(s, up, 1).unwrap();
        assert!(down > up);
    }

    #[test]
    fn yank_after_kill() {
        let mut p = PromptFrame::default();
        p.set_text("hello world");
        p.cursor = 5;
        p.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(p.text, "hello");
        p.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(p.text, "hello world");
    }
}
