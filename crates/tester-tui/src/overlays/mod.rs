mod branch;
mod chat_sessions;
mod diff;
mod manual;
mod plan;
mod session_list;

pub use branch::BranchOverlay;
pub use chat_sessions::ChatSessionsOverlay;
pub use diff::DiffOverlay;
pub use manual::ManualOverlay;
pub use plan::{PlanDecision, PlanOverlay};
pub use session_list::SessionListOverlay;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::theme::Theme;

#[derive(Debug)]
pub enum Overlay {
    Manual(ManualOverlay),
    Recent(SessionListOverlay),
    ChatSessions(ChatSessionsOverlay),
    Diff(DiffOverlay),
    Branch(BranchOverlay),
    Plan(PlanOverlay),
}

#[derive(Debug, Clone)]
pub enum OverlayAction {
    None,
    Close,
    SwitchSession(std::path::PathBuf),
    ResumeChatSession(String),
    NewChatSession,
    SwitchBranch(String),
    PlanDone(PlanDecision),
}

impl Overlay {
    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        match self {
            Overlay::Manual(o) => o.draw(frame, area, theme),
            Overlay::Recent(o) => o.draw(frame, area, theme),
            Overlay::ChatSessions(o) => o.draw(frame, area, theme),
            Overlay::Diff(o) => o.draw(frame, area, theme),
            Overlay::Branch(o) => o.draw(frame, area, theme),
            Overlay::Plan(o) => o.draw(frame, area, theme),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match self {
            Overlay::Manual(o) => o.handle_key(key),
            Overlay::Recent(o) => o.handle_key(key),
            Overlay::ChatSessions(o) => o.handle_key(key),
            Overlay::Diff(o) => o.handle_key(key),
            Overlay::Branch(o) => o.handle_key(key),
            Overlay::Plan(o) => o.handle_key(key),
        }
    }
}
