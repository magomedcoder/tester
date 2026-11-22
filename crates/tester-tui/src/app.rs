use super::TuiOptions;
use super::event::Event;
use super::recents::Recents;
use super::screens::Screen;
use super::screens::picker::{PickerAction, PickerScreen};
use super::screens::session::{SessionAction, SessionScreen};
use super::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tester_core::i18n::{self, Msg};

#[derive(Debug, Clone)]
pub struct WorkspaceTab {
    pub path: PathBuf,
    pub title: String,
    pub pending: bool,
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub dry_run: bool,
    pub session_id: Option<String>,
    pub project: Option<PathBuf>,
    pub theme: Theme,
    pub tabs: Vec<WorkspaceTab>,
    pub tab_idx: usize,
    ctrl_c_armed: Option<Instant>,
    external_editor: Option<String>,
    bell: bool,
    last_title: String,
}

impl App {
    pub fn new(opts: TuiOptions) -> Self {
        let mut app = Self {
            screen: Screen::Picker(PickerScreen::new(None)),
            should_quit: false,
            dry_run: opts.dry_run,
            session_id: opts.session_id.clone(),
            project: None,
            theme: Theme::default(),
            tabs: Vec::new(),
            tab_idx: 0,
            ctrl_c_armed: None,
            external_editor: None,
            bell: false,
            last_title: "tester".into(),
        };
        if let Some(path) = opts.path {
            app.set_project(path);
        } else if let Some(last) = Recents::load().last_root.filter(|p| p.is_dir()) {
            app.set_project(last);
        } else {
            app.screen = Screen::Picker(PickerScreen::new(None));
        }
        app.session_id = None;
        app
    }

    fn set_project(&mut self, path: PathBuf) {
        let mut recents = Recents::load();
        recents.push(&path);
        let _ = recents.save();
        let session = SessionScreen::open(path, self.dry_run, self.session_id.clone());
        self.project = Some(session.root().to_path_buf());
        let title = self
            .project
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("session")
            .to_string();
        if let Some(existing) = self
            .tabs
            .iter()
            .position(|t| Some(&t.path) == self.project.as_ref())
        {
            self.tab_idx = existing;
        } else {
            self.tabs.push(WorkspaceTab {
                path: self.project.clone().unwrap(),
                title,
                pending: false,
            });
            self.tab_idx = self.tabs.len() - 1;
        }
        self.screen = Screen::Session(Box::new(session));
    }

    pub fn window_title(&mut self) -> Option<String> {
        let title = match &self.screen {
            Screen::Session(s) if s.is_busy() => {
                i18n::tr(Msg::WinTitleBusy, &[("model", s.model_name())])
            }
            Screen::Session(_) => i18n::t(Msg::WinTitleIdle).into(),
            Screen::Picker(_) => i18n::t(Msg::WinTitlePicker).into(),
        };
        if title != self.last_title {
            self.last_title = title.clone();
            Some(title)
        } else {
            None
        }
    }

    pub fn take_external_editor(&mut self) -> Option<String> {
        self.external_editor.take()
    }

    pub fn apply_external_editor(&mut self, text: String) {
        if let Screen::Session(s) = &mut self.screen {
            s.apply_external_editor(text);
        }
    }

    pub fn status_notice(&mut self, msg: String) {
        if let Screen::Session(s) = &mut self.screen {
            s.push_status(msg);
        }
    }

    pub fn consume_bell(&mut self) -> bool {
        let b = self.bell;
        self.bell = false;
        b
    }

    pub fn cancel_active_job(&mut self) {
        if let Screen::Session(session) = &mut self.screen {
            session.cancel_job();
        }
    }

    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Paste(s) => {
                if let Screen::Session(session) = &mut self.screen {
                    session.handle_paste(&s);
                } else if let Screen::Picker(picker) = &mut self.screen {
                    for c in s.chars() {
                        if !c.is_control() {
                            let _ = picker
                                .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
                        }
                    }
                }
            }
            Event::Resize => {}
            Event::Tick => {
                if let Screen::Session(session) = &mut self.screen {
                    session.tick();
                    if let Some(i) = self.tabs.get_mut(self.tab_idx) {
                        i.pending = session.is_busy();
                    }
                    if session.consume_bell() {
                        self.bell = true;
                    }
                    if session.wants_external_editor() {
                        self.external_editor = Some(session.composer_text());
                    }
                }
            }
        }
    }

    fn handle_workspace_keys(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if (ctrl && key.code == KeyCode::PageUp) || (alt && key.code == KeyCode::Char(',')) {
            if self.tab_idx > 0 {
                self.tab_idx -= 1;
                if let Some(t) = self.tabs.get(self.tab_idx) {
                    self.set_project(t.path.clone());
                }
            }
            return true;
        }
        if (ctrl && key.code == KeyCode::PageDown) || (alt && key.code == KeyCode::Char('.')) {
            if self.tab_idx + 1 < self.tabs.len() {
                self.tab_idx += 1;
                if let Some(t) = self.tabs.get(self.tab_idx) {
                    self.set_project(t.path.clone());
                }
            }
            return true;
        }
        if alt && key.code == KeyCode::Insert {
            self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
            return true;
        }
        if alt && key.code == KeyCode::Delete {
            if !self.tabs.is_empty() {
                self.tabs.remove(self.tab_idx);
                if self.tabs.is_empty() {
                    self.project = None;
                    self.screen = Screen::Picker(PickerScreen::new(None));
                } else {
                    self.tab_idx = self.tab_idx.min(self.tabs.len() - 1);
                    let path = self.tabs[self.tab_idx].path.clone();
                    self.set_project(path);
                }
            }
            return true;
        }
        false
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::SHIFT)
            && key.code == KeyCode::Char('c')
        {
            let now = Instant::now();
            if let Some(t) = self.ctrl_c_armed {
                if now.duration_since(t) < Duration::from_millis(800) {
                    self.cancel_active_job();
                    self.should_quit = true;
                    return;
                }
            }
            self.ctrl_c_armed = Some(now);
            if let Screen::Session(session) = &mut self.screen {
                match session.handle_key(key) {
                    SessionAction::Quit => {
                        self.cancel_active_job();
                        self.should_quit = true;
                    }
                    SessionAction::OpenPicker => {
                        self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
                    }
                    SessionAction::None => {}
                    SessionAction::ExternalEditor => {
                        self.external_editor = Some(session.composer_text());
                    }
                    SessionAction::OpenPath(p) => self.set_project(p),
                }
            } else {
                self.should_quit = true;
            }
            return;
        }

        if self.handle_workspace_keys(key) {
            return;
        }

        match &mut self.screen {
            Screen::Picker(picker) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
                    return;
                }
                match picker.handle_key(key) {
                    PickerAction::None => {}
                    PickerAction::Cancel => {
                        if let Some(path) = self.project.clone() {
                            self.set_project(path);
                        } else if matches!(key.code, KeyCode::Char('q')) {
                            self.should_quit = true;
                        }
                    }
                    PickerAction::Selected(path) => self.set_project(path),
                }
            }
            Screen::Session(session) => match session.handle_key(key) {
                SessionAction::None => {}
                SessionAction::Quit => {
                    self.cancel_active_job();
                    self.should_quit = true;
                }
                SessionAction::OpenPicker => {
                    self.screen = Screen::Picker(PickerScreen::new(self.project.clone()));
                }
                SessionAction::ExternalEditor => {
                    self.external_editor = Some(session.composer_text());
                }
                SessionAction::OpenPath(p) => self.set_project(p),
            },
        }
    }
}
