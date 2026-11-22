use std::io::{self, Write};
use std::time::Instant;

pub struct Progress {
    started: Instant,
    last: String,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            last: String::new(),
        }
    }

    pub fn step(&mut self, label: &str) {
        self.last = label.to_string();
        if tester_core::tty::is_active()
            || crossterm::terminal::is_raw_mode_enabled().unwrap_or(false)
            || !io::IsTerminal::is_terminal(&io::stderr())
        {
            return;
        }
        let secs = self.started.elapsed().as_secs();
        let spinner = match secs % 4 {
            0 => "|",
            1 => "/",
            2 => "-",
            _ => "\\",
        };
        let _ = writeln!(io::stderr(), "{spinner} [{secs:>3}s] agent :: {label}");
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}
