use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn str_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn char_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

pub fn truncate_end(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if str_width(s) <= max {
        return s.to_string();
    }
    if max == 1 {
        return "...".into();
    }
    let mut out = String::new();
    let mut w = 0usize;
    let budget = max - 1;
    for ch in s.chars() {
        let cw = char_width(ch);
        if w + cw > budget {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

pub fn truncate_middle(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if str_width(s) <= max {
        return s.to_string();
    }
    if max <= 3 {
        return truncate_end(s, max);
    }
    let keep = max - 1;
    let head_budget = keep / 2;
    let tail_budget = keep - head_budget;

    let mut head = String::new();
    let mut hw = 0usize;
    for ch in s.chars() {
        let cw = char_width(ch);
        if hw + cw > head_budget {
            break;
        }
        head.push(ch);
        hw += cw;
    }

    let mut tail_chars: Vec<char> = Vec::new();
    let mut tw = 0usize;
    for ch in s.chars().rev() {
        let cw = char_width(ch);
        if tw + cw > tail_budget {
            break;
        }
        tail_chars.push(ch);
        tw += cw;
    }
    tail_chars.reverse();

    let mut out = head;
    out.push('…');
    out.extend(tail_chars);
    out
}

pub fn pad_to_width(s: &str, target: usize) -> String {
    let mut out = s.to_string();
    let mut w = str_width(&out);
    while w < target {
        out.push(' ');
        w += 1;
    }
    out
}

pub fn wrap_display(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut line = String::new();
    let mut line_w = 0usize;

    let push_line = |line: &mut String, line_w: &mut usize, out: &mut Vec<String>| {
        out.push(std::mem::take(line));
        *line_w = 0;
    };

    for ch in s.chars() {
        if ch == '\n' {
            push_line(&mut line, &mut line_w, &mut out);
            continue;
        }
        let cw = char_width(ch);
        if cw == 0 {
            line.push(ch);
            continue;
        }
        if line_w > 0 && line_w + cw > width {
            push_line(&mut line, &mut line_w, &mut out);
        }
        if line_w == 0 && cw > width {
            line.push(ch);
            push_line(&mut line, &mut line_w, &mut out);
            continue;
        }
        line.push(ch);
        line_w += cw;
    }
    out.push(line);
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_wrap() {
        assert_eq!(wrap_display("abcdef", 3), vec!["abc", "def"]);
        assert_eq!(wrap_display("ab\ncd", 10), vec!["ab", "cd"]);
    }

    #[test]
    fn wide_char_wrap() {
        let s = "测测测";
        let lines = wrap_display(s, 4);
        assert_eq!(lines.len(), 2);
        assert_eq!(str_width(&lines[0]), 4);
        assert_eq!(str_width(&lines[1]), 2);
    }

    #[test]
    fn truncate_middle_display() {
        assert_eq!(truncate_middle("abcdef", 10), "abcdef");
        let t = truncate_middle("abcdefghij", 7);
        assert!(str_width(&t) <= 7);
        assert!(t.contains('…'));
    }

    #[test]
    fn truncate_end_wide() {
        let s = "测试测试";
        let t = truncate_end(s, 5);
        assert!(str_width(&t) <= 5);
        assert!(t.ends_with('…'));
    }
}
