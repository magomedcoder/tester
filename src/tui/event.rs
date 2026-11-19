use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Resize,
    Tick,
}

pub fn poll(timeout: Duration) -> Result<bool> {
    Ok(event::poll(timeout)?)
}

pub fn read() -> Result<Event> {
    match event::read()? {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => Ok(Event::Key(key)),
        CrosstermEvent::Resize(_, _) => Ok(Event::Resize),
        _ => Ok(Event::Tick),
    }
}
