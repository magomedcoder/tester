mod app;
mod event;
mod recents;
mod screens;
mod ui;

use anyhow::{Result, bail};
use app::App;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TuiOptions {
    pub path: Option<PathBuf>,
    pub dry_run: bool,
    pub resume: bool,
}

pub fn run(opts: TuiOptions) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("{}", crate::i18n::t(crate::i18n::Msg::TuiNeedsTty));
    }

    let mut app = App::new(opts);
    let mut terminal = setup()?;
    let result = run_loop(&mut terminal, &mut app);
    restore()?;
    result
}

fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(100))? {
            let ev = event::read()?;
            app.handle(ev);
        } else {
            app.handle(event::Event::Tick);
        }
    }
    Ok(())
}
