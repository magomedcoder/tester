use super::TuiOptions;
use super::event::Event;
use super::screens::{Screen, home::HomeScreen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub dry_run: bool,
    pub resume: bool,
}

impl App {
    pub fn new(opts: TuiOptions) -> Self {
        Self {
            screen: Screen::Home(HomeScreen::new(opts.path)),
            should_quit: false,
            dry_run: opts.dry_run,
            resume: opts.resume,
        }
    }

    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize => {}
            Event::Tick => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => match &mut self.screen {
                Screen::Home(home) => home.handle_key(key),
            },
        }
    }
}
