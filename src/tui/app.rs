use super::TuiOptions;
use super::event::Event;
use super::screens::home::{HomeAction, HomeScreen};
use super::screens::picker::{PickerAction, PickerScreen};
use super::screens::Screen;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub dry_run: bool,
    pub resume: bool,
    pub project: Option<PathBuf>,
}

impl App {
    pub fn new(opts: TuiOptions) -> Self {
        let mut app = Self {
            screen: Screen::Home(HomeScreen::new(None)),
            should_quit: false,
            dry_run: opts.dry_run,
            resume: opts.resume,
            project: None,
        };
        if let Some(path) = opts.path {
            app.set_project(path);
        } else {
            app.screen = Screen::Picker(PickerScreen::new(None));
        }
        app
    }

    fn set_project(&mut self, path: PathBuf) {
        let mut home = HomeScreen::new(None);
        home.set_project(path);
        self.project = home.path.clone();
        self.screen = Screen::Home(home);
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

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
            self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
            return;
        }

        match &mut self.screen {
            Screen::Home(home) => {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    self.should_quit = true;
                    return;
                }
                if matches!(home.handle_key(key), HomeAction::OpenPicker) {
                    let start = self.project.clone();
                    self.screen = Screen::Picker(PickerScreen::new(start));
                }
            }
            Screen::Picker(picker) => match picker.handle_key(key) {
                PickerAction::None => {}
                PickerAction::Cancel => {
                    let path = self.project.clone();
                    self.screen = Screen::Home(HomeScreen::new(path));
                }
                PickerAction::Selected(path) => {
                    self.set_project(path);
                }
            },
        }
    }
}
