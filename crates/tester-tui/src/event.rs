use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Paste(String),
    Resize,
    Tick,
}

pub fn poll(timeout: Duration) -> Result<bool> {
    Ok(event::poll(timeout)?)
}

pub fn read() -> Result<Event> {
    match event::read()? {
        CrosstermEvent::Key(key)
            if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
        {
            Ok(Event::Key(key))
        }
        CrosstermEvent::Paste(s) => Ok(Event::Paste(s)),
        CrosstermEvent::Resize(_, _) => Ok(Event::Resize),
        _ => Ok(Event::Tick),
    }
}
