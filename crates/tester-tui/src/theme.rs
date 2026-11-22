use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color,
    pub bg_raised: Color,
    pub bg_popup: Color,
    pub bg_popup_sel: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub fg_faint: Color,
    pub accent: Color,
    pub border: Color,
    pub border_focused: Color,
    pub user: Color,
    pub assistant: Color,
    pub tool: Color,
    pub system: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(12, 12, 12),
            bg_raised: Color::Rgb(28, 28, 28),
            bg_popup: Color::Rgb(38, 38, 42),
            bg_popup_sel: Color::Rgb(58, 58, 64),
            fg: Color::Rgb(245, 245, 245),
            fg_muted: Color::Rgb(140, 140, 140),
            fg_faint: Color::Rgb(72, 72, 72),
            accent: Color::Rgb(255, 255, 255),
            border: Color::Rgb(48, 48, 48),
            border_focused: Color::Rgb(220, 220, 220),
            user: Color::Rgb(255, 255, 255),
            assistant: Color::Rgb(210, 210, 210),
            tool: Color::Rgb(150, 150, 150),
            system: Color::Rgb(110, 110, 110),
            success: Color::Rgb(200, 200, 200),
            warning: Color::Rgb(170, 170, 170),
            error: Color::Rgb(255, 255, 255),
            info: Color::Rgb(160, 160, 160),
        }
    }
}

impl Theme {
    pub fn base(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.fg_muted).bg(self.bg)
    }

    pub fn faint(&self) -> Style {
        Style::default().fg(self.fg_faint).bg(self.bg)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent).bg(self.bg)
    }

    pub fn brand(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn role_style(&self, role: &str) -> Style {
        let fg = match role {
            "you" | "user" => self.user,
            "assistant" | "tester" => self.assistant,
            "tool" => self.tool,
            _ => self.system,
        };
        Style::default()
            .fg(fg)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_accent(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .bg(self.bg_raised)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error).bg(self.bg)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success).bg(self.bg)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning).bg(self.bg)
    }

    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info).bg(self.bg)
    }

    pub fn border_block(&self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.border_focused
        } else {
            self.border
        })
    }

    pub fn raised(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg_raised)
    }

    pub fn popup(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg_popup)
    }

    pub fn popup_selected(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg_popup_sel)
            .add_modifier(Modifier::BOLD)
    }

    pub fn diff_added(&self) -> Style {
        Style::default().fg(self.success).bg(self.bg)
    }

    pub fn diff_removed(&self) -> Style {
        Style::default().fg(self.error).bg(self.bg)
    }
}

pub fn truncate_middle(s: &str, width: usize) -> String {
    super::width::truncate_middle(s, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::width::str_width;

    #[test]
    fn truncate_middle_keeps_ends() {
        assert_eq!(truncate_middle("abcdef", 10), "abcdef");
        assert!(str_width(&truncate_middle("abcdefghij", 7)) <= 7);
        assert!(truncate_middle("/very/long/path/to/file.rs", 16).contains('…'));
    }

    #[test]
    fn role_styles_differ() {
        let t = Theme::default();
        assert_ne!(t.role_style("you").fg, t.role_style("tool").fg);
    }
}
