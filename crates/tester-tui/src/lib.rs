#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if,
    )]

mod app;
mod clipboard;
mod composer;
mod event;
mod files;
mod health;
mod markdown;
mod overlays;
mod popup;
mod recents;
mod screens;
pub mod theme;
mod transcript;
mod ui;
mod width;

use anyhow::{Result, bail};
use app::App;
use crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::Duration;

fn set_active(v: bool) {
    tester_core::tty::set_active(v);
}

#[derive(Debug, Clone)]
pub struct TuiOptions {
    pub path: Option<PathBuf>,
    pub dry_run: bool,
    pub session_id: Option<String>,
}

pub struct TerminalUiGuard;

impl TerminalUiGuard {
    fn keyboard_flags() -> KeyboardEnhancementFlags {
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
    }

    fn set_alternate_scroll(enable: bool) {
        let seq = if enable { "\x1b[?1007h" } else { "\x1b[?1007l" };
        let _ = write!(io::stdout(), "{seq}");
        let _ = io::stdout().flush();
    }

    pub fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableBracketedPaste,
            SetTitle("tester")
        )?;
        Self::set_alternate_scroll(true);
        let _ = execute!(stdout, PushKeyboardEnhancementFlags(Self::keyboard_flags()));
        set_active(true);
        Ok(Terminal::new(CrosstermBackend::new(stdout))?)
    }

    pub fn restore() {
        set_active(false);
        Self::set_alternate_scroll(false);
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableBracketedPaste,
            SetTitle("")
        );
    }

    pub fn suspend(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        set_active(false);
        Self::set_alternate_scroll(false);
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableBracketedPaste
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn resume(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            SetTitle("tester")
        )?;
        Self::set_alternate_scroll(true);
        let _ = execute!(
            terminal.backend_mut(),
            PushKeyboardEnhancementFlags(Self::keyboard_flags())
        );
        set_active(true);
        terminal.clear()?;
        Ok(())
    }

    pub fn set_title(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, title: &str) {
        let _ = execute!(terminal.backend_mut(), SetTitle(title));
    }

    pub fn bell(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        let _ = write!(terminal.backend_mut(), "\x07");
        let _ = terminal.backend_mut().flush();
    }
}

struct ActiveTerminal {
    _terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restored: bool,
}

impl ActiveTerminal {
    fn new(terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Self {
        Self {
            _terminal: terminal,
            restored: false,
        }
    }

    fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self._terminal
    }

    fn restore_now(&mut self) {
        if !self.restored {
            self.restored = true;
            TerminalUiGuard::restore();
        }
    }
}

impl Drop for ActiveTerminal {
    fn drop(&mut self) {
        self.restore_now();
    }
}

pub fn run(opts: TuiOptions) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!(
            "{}",
            tester_core::i18n::t(tester_core::i18n::Msg::TuiNeedsTty)
        );
    }

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        TerminalUiGuard::restore();
        prev_hook(info);
    }));

    let mut app = App::new(opts);
    let mut active = ActiveTerminal::new(TerminalUiGuard::setup()?);
    let result = (|| {
        while !app.should_quit {
            if let Some(title) = app.window_title() {
                TerminalUiGuard::set_title(active.terminal(), &title);
            }
            active.terminal().draw(|frame| ui::draw(frame, &mut app))?;
            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                if matches!(ev, event::Event::Resize) {
                    let _ = active.terminal().clear();
                }
                if let Some(initial) = app.take_external_editor() {
                    TerminalUiGuard::suspend(active.terminal())?;
                    let edited = composer::edit_external(&initial);
                    TerminalUiGuard::resume(active.terminal())?;
                    match edited {
                        Ok(text) => app.apply_external_editor(text),
                        Err(e) => app.status_notice(tester_core::i18n::tr(
                            tester_core::i18n::Msg::EditorFailed,
                            &[("err", &format!("{e:#}"))],
                        )),
                    }
                    continue;
                }
                app.handle(ev);
                if app.consume_bell() {
                    TerminalUiGuard::bell(active.terminal());
                }
            } else {
                app.handle(event::Event::Tick);
            }
        }
        app.cancel_active_job();
        Ok(())
    })();
    active.restore_now();
    result
}
