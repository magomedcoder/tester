use super::TuiOptions;
use super::event::Event;
use super::screens::home::{HomeAction, HomeScreen};
use super::screens::picker::{PickerAction, PickerScreen};
use super::screens::session::{SessionAction, SessionScreen};
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
        let session = SessionScreen::open(path, self.dry_run, self.resume);
        self.project = Some(session.root().to_path_buf());
        self.screen = Screen::Session(session);
    }

    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize => {}
            Event::Tick => {
                if let Screen::Session(session) = &mut self.screen {
                    session.tick();
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if let Screen::Session(session) = &mut self.screen {
                match session.handle_key(key) {
                    SessionAction::Quit => self.should_quit = true,
                    SessionAction::OpenPicker => {
                        self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
                    }
                    SessionAction::None => {}
                }
                return;
            }
            self.should_quit = true;
            return;
        }

        match &mut self.screen {
            Screen::Home(home) => {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    self.should_quit = true;
                    return;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o')
                    || matches!(home.handle_key(key), HomeAction::OpenPicker)
                {
                    let start = self.project.clone();
                    self.screen = Screen::Picker(PickerScreen::new(start));
                } else if key.code == KeyCode::Enter {
                    if let Some(path) = self.project.clone() {
                        self.set_project(path);
                    }
                }
            }
            Screen::Picker(picker) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
                    return;
                }
                match picker.handle_key(key) {
                    PickerAction::None => {}
                    PickerAction::Cancel => {
                        if let Some(path) = self.project.clone() {
                            self.set_project(path);
                        } else {
                            self.screen = Screen::Home(HomeScreen::new(None));
                        }
                    }
                    PickerAction::Selected(path) => self.set_project(path),
                }
            }
            Screen::Session(session) => match session.handle_key(key) {
                SessionAction::None => {}
                SessionAction::Quit => self.should_quit = true,
                SessionAction::OpenPicker => {
                    self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
                }
            },
        }
    }
}
