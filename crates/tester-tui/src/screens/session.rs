use crate::clipboard;
use crate::composer::{PromptFrame, normalize_paste};
use crate::files::{active_at_needle, extract_at_specs, list_at_candidates, read_attach_snippet};
use crate::health::HealthProbe;
use crate::overlays::{
    BranchOverlay, ChatSessionsOverlay, DiffOverlay, ManualOverlay, ModelOverlay, Overlay,
    OverlayAction, PlanDecision, PlanOverlay, SecurityOverlay, SessionListOverlay,
};
use crate::popup::{Popup, PopupItem, draw_popup, place_above, popup_height};
use crate::theme::{Theme, truncate_middle};
use crate::transcript::TranscriptView;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tester_agent::facade::{build_and_save_plan, build_plan};
use tester_agent::session_store::{self, PruneSpec};
use tester_agent::{
    AgentOutcome, AgentSession, PermissionPolicy, RunOneShotOpts, open_context_ex,
    open_context_with_policy, run_one_shot_ex,
};
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
use tester_core::model::TestKind;
use tester_providers::Usage;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const SLASH_CMDS: &[&str] = &[
    "help",
    "scan",
    "plan",
    "run",
    "clear",
    "audit",
    "exit",
    "grep",
    "symbol",
    "refs",
    "search",
    "index",
    "map",
    "list_files",
    "show_file",
    "manual",
    "recent",
    "session",
    "prune",
    "diff",
    "branch",
    "propose_plan",
    "security",
    "model",
];

fn slash_desc(cmd: &str) -> &'static str {
    match cmd {
        "help" => i18n::t(Msg::SlashHelp),
        "scan" => i18n::t(Msg::SlashScan),
        "plan" => i18n::t(Msg::SlashPlan),
        "run" => i18n::t(Msg::SlashRun),
        "clear" => i18n::t(Msg::SlashClear),
        "audit" => i18n::t(Msg::SlashAudit),
        "exit" => i18n::t(Msg::SlashExit),
        "grep" => i18n::t(Msg::SlashGrep),
        "symbol" => i18n::t(Msg::SlashSymbol),
        "refs" => i18n::t(Msg::SlashRefs),
        "search" => i18n::t(Msg::SlashSearch),
        "index" => i18n::t(Msg::SlashIndex),
        "map" => i18n::t(Msg::SlashMap),
        "list_files" => i18n::t(Msg::SlashListFiles),
        "show_file" => i18n::t(Msg::SlashShowFile),
        "manual" => i18n::t(Msg::SlashManual),
        "recent" => i18n::t(Msg::SlashRecent),
        "session" => i18n::t(Msg::SlashSession),
        "prune" => i18n::t(Msg::SlashPrune),
        "diff" => i18n::t(Msg::SlashDiff),
        "branch" => i18n::t(Msg::SlashBranch),
        "propose_plan" => i18n::t(Msg::SlashProposePlan),
        "security" => i18n::t(Msg::SlashSecurity),
        "model" => i18n::t(Msg::SlashModel),
        _ => "",
    }
}

#[derive(Debug, Clone)]
pub enum SessionAction {
    None,
    Quit,
    OpenPicker,
    ExternalEditor,
    OpenPath(PathBuf),
}

struct PermReq {
    tool: String,
    preview: String,
    reply: SyncSender<bool>,
}

struct Job {
    rx: Receiver<std::result::Result<AgentOutcome, String>>,
    cancel: Arc<AtomicBool>,
    stream_buf: Arc<Mutex<String>>,
    perm_rx: Receiver<PermReq>,
    started: Instant,
}

pub struct SessionScreen {
    path: PathBuf,
    session_id: String,
    model: String,
    base_url: String,
    dry_run: bool,
    auto_approve: bool,
    prompt: PromptFrame,
    transcript: TranscriptView,
    status: String,
    status_error: bool,
    job: Option<Job>,
    slash_selected: usize,
    at_selected: usize,
    at_files: Vec<String>,
    pending_perm: Option<PermReq>,
    streaming_idx: Option<usize>,
    overlay: Option<Overlay>,
    health: HealthProbe,
    tick_n: u64,
    esc_armed: Option<Instant>,
    ctrl_x_armed: Option<Instant>,
    ext_editor_pending: bool,
    bell_pending: bool,
    bell_on_done: bool,
    last_usage: Usage,
    ctx_limit: u32,
    plan_stash: String,
}

impl SessionScreen {
    pub fn root(&self) -> &std::path::Path {
        &self.path
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn open(path: PathBuf, dry_run: bool, session_id: Option<String>) -> Self {
        let ctx_limit = Config::load()
            .map(|c| (c.run.max_context_chars / 4).max(1) as u32)
            .unwrap_or(6000);
        let mut screen = Self {
            path: path.clone(),
            session_id: String::new(),
            model: String::new(),
            base_url: String::new(),
            dry_run,
            auto_approve: false,
            prompt: PromptFrame::default(),
            transcript: TranscriptView::new(),
            status: String::new(),
            status_error: false,
            job: None,
            slash_selected: 0,
            at_selected: 0,
            at_files: Vec::new(),
            pending_perm: None,
            streaming_idx: None,
            overlay: None,
            health: HealthProbe::new(""),
            tick_n: 0,
            esc_armed: None,
            ctrl_x_armed: None,
            ext_editor_pending: false,
            bell_pending: false,
            bell_on_done: true,
            last_usage: Usage::default(),
            ctx_limit,
            plan_stash: String::new(),
        };
        match open_context_with_policy(&path, dry_run, false, true) {
            Ok(ctx) => {
                screen.model = ctx.config.llm.model.clone();
                screen.base_url = ctx.config.llm.base_url().to_string();
                screen.auto_approve = ctx.config.run.agent_auto_approve;
                screen.path = ctx.profile.root.clone();
                screen.health = HealthProbe::new(&screen.base_url);
                screen.ctx_limit = (ctx.config.run.max_context_chars / 4).max(1) as u32;
                let loaded = if let Some(id) = session_id {
                    session_store::resolve_id(&id)
                        .and_then(|id| session_store::load(&id))
                        .and_then(|(meta, session)| {
                            session_store::set_active(&screen.path, &meta.id)?;
                            Ok((meta, session))
                        })
                } else {
                    session_store::open_for_workspace(&screen.path)
                };
                match loaded {
                    Ok((meta, session)) => {
                        screen.session_id = meta.id.clone();
                        screen.hydrate_from_session(&session);
                        screen.status = format!(
                            "{}{}",
                            &meta.id[..meta.id.len().min(8)],
                            if dry_run {
                                i18n::t(Msg::SessionStatusDryRun)
                            } else if screen.auto_approve {
                                i18n::t(Msg::SessionStatusAuto)
                            } else {
                                i18n::t(Msg::SessionStatusBlocked)
                            }
                        );
                    }
                    Err(err) => {
                        screen.push_system(i18n::tr(
                            Msg::SessionOpenFailed,
                            &[("err", &format!("{err:#}"))],
                        ));
                        screen.status_error = true;
                    }
                }
                if screen.model.trim().is_empty() {
                    screen.status = i18n::t(Msg::ModelSelectHint).into();
                    screen.open_model_overlay(None);
                }
                screen.update_ghost();
            }
            Err(err) => {
                screen.transcript.push(
                    "system",
                    format!("{}: {err:#}", i18n::t(Msg::SessionFailedOpen)),
                );
                screen.status = i18n::t(Msg::SessionFixHint).into();
                screen.status_error = true;
            }
        }
        screen
    }

    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn cancel_job(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::SeqCst);
        }
    }

    pub fn consume_bell(&mut self) -> bool {
        let b = self.bell_pending;
        self.bell_pending = false;
        b
    }

    pub fn wants_external_editor(&mut self) -> bool {
        if self.ext_editor_pending {
            self.ext_editor_pending = false;
            true
        } else {
            false
        }
    }

    pub fn composer_text(&self) -> String {
        self.prompt.text.clone()
    }

    pub fn apply_external_editor(&mut self, text: String) {
        self.prompt.set_text(text);
        self.refresh_at_files();
        self.update_ghost();
    }

    pub fn push_status(&mut self, msg: String) {
        self.status = msg;
        self.status_error = false;
    }

    pub fn conn_glyph(&self) -> &str {
        self.health.state().glyph()
    }

    pub fn ctx_gauge(&self) -> (u32, u32) {
        (self.last_usage.total(), self.ctx_limit)
    }

    pub fn badges(&self) -> String {
        let mut parts = Vec::new();
        if self.job.is_some() {
            if self.streaming_idx.is_some() {
                parts.push(i18n::t(Msg::BadgeThinking));
            } else {
                parts.push(i18n::t(Msg::BadgeWorking));
            }
        } else if self.pending_perm.is_some() {
            parts.push(i18n::t(Msg::BadgePerm));
        }
        if matches!(self.overlay, Some(Overlay::Plan(_))) {
            parts.push(i18n::t(Msg::BadgePlan));
        }
        parts.join("")
    }

    pub fn handle_paste(&mut self, s: &str) {
        if self.overlay.is_some() || self.pending_perm.is_some() {
            return;
        }
        let s = normalize_paste(s);
        if s.is_empty() {
            return;
        }
        self.prompt.insert_str(&s);
        self.slash_selected = 0;
        self.refresh_at_files();
        self.update_ghost();
    }

    fn paste_from_clipboard(&mut self) -> bool {
        if let Some(s) = clipboard::read_paste() {
            self.handle_paste(&s);
            true
        } else {
            false
        }
    }

    fn hydrate_from_session(&mut self, session: &AgentSession) {
        self.transcript.clear();
        for msg in &session.messages {
            if msg.role == "system" {
                continue;
            }
            let role: &'static str = match msg.role.as_str() {
                "user" => "you",
                "assistant" => "assistant",
                "tool" => "tool",
                _ => "system",
            };
            let mut body = msg.content.clone().unwrap_or_default();
            if let Some(calls) = &msg.tool_calls {
                for c in calls {
                    if !body.is_empty() {
                        body.push('\n');
                    }
                    body.push_str(&format!("-> {}({})", c.function.name, c.function.arguments));
                }
            }
            if body.trim().is_empty() {
                continue;
            }
            self.transcript.push(role, body);
        }
    }

    pub fn tick(&mut self) {
        self.tick_n = self.tick_n.wrapping_add(1);
        self.health.tick();
        if self.pending_perm.is_none() {
            if let Some(job) = &self.job {
                if let Ok(req) = job.perm_rx.try_recv() {
                    self.pending_perm = Some(req);
                    self.status = i18n::t(Msg::PermPrompt).into();
                    self.status_error = false;
                }
            }
        }

        let Some(job) = &self.job else {
            return;
        };

        if let Ok(buf) = job.stream_buf.lock() {
            if !buf.is_empty() {
                let text = buf.clone();
                drop(buf);
                if let Some(idx) = self.streaming_idx {
                    if let Some(e) = self.transcript.entries.get_mut(idx) {
                        e.text = text;
                        self.transcript.invalidate();
                    }
                } else {
                    self.transcript.push("assistant", text);
                    self.streaming_idx = Some(self.transcript.entries.len().saturating_sub(1));
                }
            }
        }

        let elapsed = job.started.elapsed().as_secs();
        let spin = SPINNER[(self.tick_n as usize) % SPINNER.len()];

        match job.rx.try_recv() {
            Ok(Ok(outcome)) => {
                let started = job.started;
                self.job = None;
                self.pending_perm = None;
                let text = if outcome.final_text.trim().is_empty() {
                    i18n::t(Msg::SessionEmptyResponse).into()
                } else {
                    outcome.final_text
                };
                if let Some(idx) = self.streaming_idx.take() {
                    if let Some(e) = self.transcript.entries.get_mut(idx) {
                        e.text = text;
                        self.transcript.invalidate();
                    }
                } else {
                    self.transcript.push("assistant", text);
                }
                let tok = outcome.usage.total();
                self.last_usage = outcome.usage;
                let status = if outcome.cancelled {
                    i18n::t(Msg::SessionOutcomeCancelled)
                } else {
                    i18n::t(Msg::SessionOutcomeOk)
                };
                self.status = format!(
                    "{}tokens≈{tok}",
                    i18n::tr(
                        Msg::SessionOutcomeLine,
                        &[
                            ("turns", &outcome.turns.to_string()),
                            ("tools", &outcome.tool_calls.to_string()),
                            ("status", status),
                        ],
                    )
                );
                self.status_error = outcome.cancelled;
                if self.bell_on_done && started.elapsed() > Duration::from_secs(3) {
                    self.bell_pending = true;
                }
                self.transcript.stick_bottom = true;
            }
            Ok(Err(err)) => {
                self.job = None;
                self.pending_perm = None;
                self.streaming_idx = None;
                let short: String = err.chars().take(120).collect();
                self.transcript
                    .push("system", format!("{}{short}", i18n::t(Msg::ErrorPrefix)));
                self.status = i18n::t(Msg::SessionError).into();
                self.status_error = true;
            }
            Err(TryRecvError::Empty) => {
                if self.pending_perm.is_none() {
                    self.status = format!("{spin} {}  {elapsed}s", i18n::t(Msg::SessionRunning));
                    self.status_error = false;
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.pending_perm = None;
                self.status = i18n::t(Msg::SessionThreadExit).into();
                self.status_error = true;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SessionAction {
        if let Some(req) = self.pending_perm.take() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let _ = req.reply.send(true);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    let _ = req.reply.send(false);
                }
                _ => {
                    self.pending_perm = Some(req);
                }
            }
            return SessionAction::None;
        }

        if let Some(overlay) = &mut self.overlay {
            let action = overlay.handle_key(key);
            return self.apply_overlay_action(action);
        }

        let wants_paste = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('v') | KeyCode::Char('V')))
            || (key.code == KeyCode::Insert && key.modifiers.contains(KeyModifiers::SHIFT));
        if wants_paste {
            if self.paste_from_clipboard() {
                return SessionAction::None;
            }
            return SessionAction::None;
        }

        if self.job.is_some() {
            if self.handle_esc_sequence(key) {
                if let Some(job) = &self.job {
                    job.cancel.store(true, Ordering::SeqCst);
                }
                self.status = i18n::t(Msg::SessionCancelReq).into();
                return SessionAction::None;
            }
            if matches!(key.code, KeyCode::PageUp | KeyCode::Up) {
                self.transcript.scroll_up(3);
                return SessionAction::None;
            }
            if matches!(key.code, KeyCode::PageDown | KeyCode::Down) {
                self.transcript.scroll_down(3);
                return SessionAction::None;
            }
            return SessionAction::None;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
            return SessionAction::OpenPicker;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('x') {
            self.ctrl_x_armed = Some(Instant::now());
            self.status = i18n::t(Msg::StatusOpenEditor).into();
            return SessionAction::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
            if self
                .ctrl_x_armed
                .map(|t| t.elapsed() < Duration::from_secs(30))
                .unwrap_or(false)
            {
                self.ctrl_x_armed = None;
                self.ext_editor_pending = true;
                return SessionAction::ExternalEditor;
            }
        }

        if self.at_active() {
            match key.code {
                KeyCode::Up => {
                    self.at_selected = self.at_selected.saturating_sub(1);
                    return SessionAction::None;
                }
                KeyCode::Down => {
                    if !self.at_files.is_empty() {
                        self.at_selected =
                            (self.at_selected + 1).min(self.at_files.len().saturating_sub(1));
                    }
                    return SessionAction::None;
                }
                KeyCode::Tab | KeyCode::Enter => {
                    if let Some(path) = self.at_files.get(self.at_selected).cloned() {
                        self.insert_at_path(&path);
                    }
                    return SessionAction::None;
                }
                KeyCode::BackTab if self.at_active() => {
                    if !self.at_files.is_empty() {
                        self.at_selected = if self.at_selected == 0 {
                            self.at_files.len() - 1
                        } else {
                            self.at_selected - 1
                        };
                    }
                    return SessionAction::None;
                }
                KeyCode::Esc => {
                    if let Some(idx) = self.prompt.text.rfind('@') {
                        self.prompt.text.truncate(idx);
                        self.prompt.cursor = self.prompt.text.len();
                        self.refresh_at_files();
                        self.update_ghost();
                    }
                    return SessionAction::None;
                }
                _ => {}
            }
        }

        if self.slash_active() {
            match key.code {
                KeyCode::Up | KeyCode::BackTab => {
                    let n = self.slash_filtered().len();
                    if n > 0 {
                        self.slash_selected = if self.slash_selected == 0 {
                            n - 1
                        } else {
                            self.slash_selected - 1
                        };
                    }
                    self.update_ghost();
                    return SessionAction::None;
                }
                KeyCode::Down => {
                    let n = self.slash_filtered().len();
                    if n > 0 {
                        self.slash_selected = (self.slash_selected + 1) % n;
                    }
                    self.update_ghost();
                    return SessionAction::None;
                }
                KeyCode::Tab => {
                    if let Some(cmd) = self.slash_filtered().get(self.slash_selected).copied() {
                        self.prompt.set_text(format!("/{cmd} "));
                        self.slash_selected = 0;
                    }
                    self.update_ghost();
                    return SessionAction::None;
                }
                KeyCode::Esc => {
                    self.prompt.text.clear();
                    self.prompt.cursor = 0;
                    self.slash_selected = 0;
                    self.update_ghost();
                    return SessionAction::None;
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Esc {
            if self.handle_esc_sequence(key) {
                if self.prompt.text.is_empty() {
                    return SessionAction::Quit;
                }
                self.prompt.clear();
                self.refresh_at_files();
                self.update_ghost();
                return SessionAction::None;
            }
            return SessionAction::None;
        }

        if is_prompt_newline_key(&key) {
            self.prompt.insert_char('\n');
            self.update_ghost();
            return SessionAction::None;
        }

        if key.code == KeyCode::Enter {
            if self.slash_active() {
                let filtered = self.slash_filtered();
                let exact = SLASH_CMDS
                    .iter()
                    .any(|c| self.prompt.text.trim() == format!("/{c}"));
                if !exact {
                    if let Some(cmd) = filtered.get(self.slash_selected).copied() {
                        if filtered.len() == 1 || self.prompt.text.trim() == "/" {
                            self.prompt.set_text(format!("/{cmd}"));
                        } else if !self.prompt.text.contains(' ') {
                            self.prompt.set_text(format!("/{cmd} "));
                            return SessionAction::None;
                        }
                    }
                }
            }
            return self.submit_composer();
        }

        if !self.slash_active() && !self.at_needle_open() && !self.at_active() {
            match key.code {
                KeyCode::PageUp => {
                    self.transcript.scroll_up(5);
                    return SessionAction::None;
                }
                KeyCode::PageDown => {
                    self.transcript.scroll_down(5);
                    return SessionAction::None;
                }
                KeyCode::Up if key.modifiers.is_empty() => {
                    if !self.prompt.handle_key(key) {
                        self.transcript.scroll_up(3);
                    } else {
                        self.slash_selected = 0;
                        self.refresh_at_files();
                        self.update_ghost();
                    }
                    return SessionAction::None;
                }
                KeyCode::Down if key.modifiers.is_empty() => {
                    if !self.prompt.handle_key(key) {
                        self.transcript.scroll_down(3);
                    } else {
                        self.slash_selected = 0;
                        self.refresh_at_files();
                        self.update_ghost();
                    }
                    return SessionAction::None;
                }
                _ => {}
            }
        }

        if self.prompt.handle_key(key) {
            self.slash_selected = 0;
            self.refresh_at_files();
            self.update_ghost();
            return SessionAction::None;
        }

        SessionAction::None
    }

    fn handle_esc_sequence(&mut self, key: KeyEvent) -> bool {
        if key.code != KeyCode::Esc {
            self.esc_armed = None;
            return false;
        }
        let now = Instant::now();
        if let Some(t) = self.esc_armed {
            if now.duration_since(t) < Duration::from_millis(800) {
                self.esc_armed = None;
                return true;
            }
        }
        self.esc_armed = Some(now);
        false
    }

    fn apply_overlay_action(&mut self, action: OverlayAction) -> SessionAction {
        match action {
            OverlayAction::None => SessionAction::None,
            OverlayAction::Close => {
                self.overlay = None;
                SessionAction::None
            }
            OverlayAction::SwitchSession(p) => {
                self.overlay = None;
                SessionAction::OpenPath(p)
            }
            OverlayAction::ResumeChatSession(id) => {
                self.overlay = None;
                self.switch_chat_session(Some(id));
                SessionAction::None
            }
            OverlayAction::NewChatSession => {
                self.overlay = None;
                self.switch_chat_session(None);
                SessionAction::None
            }
            OverlayAction::SwitchBranch(branch) => {
                let out = Command::new("git")
                    .args(["checkout", &branch])
                    .current_dir(&self.path)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .output();
                self.overlay = None;
                let msg = match out {
                    Ok(o) if o.status.success() => {
                        i18n::tr(Msg::GitCheckoutOk, &[("branch", &branch)])
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr);
                        let err = err.trim();
                        let err = if err.is_empty() {
                            o.status.to_string()
                        } else {
                            err.to_string()
                        };
                        i18n::tr(Msg::GitCheckoutFailed, &[("err", &err)])
                    }
                    Err(e) => i18n::tr(Msg::GitCheckoutError, &[("err", &e.to_string())]),
                };
                self.push_status(msg);
                SessionAction::None
            }
            OverlayAction::PlanDone(PlanDecision::Approve) => {
                let body = self.plan_stash.clone();
                self.overlay = None;
                if !body.is_empty() {
                    self.transcript
                        .push("you", i18n::t(Msg::ProposePlanApproved));
                    self.start_agent(body);
                }
                SessionAction::None
            }
            OverlayAction::PlanDone(PlanDecision::Edit) => {
                self.overlay = None;
                if !self.plan_stash.is_empty() {
                    self.prompt.set_text(self.plan_stash.clone());
                }
                self.ext_editor_pending = true;
                SessionAction::ExternalEditor
            }
            OverlayAction::PlanDone(_) => {
                self.overlay = None;
                SessionAction::None
            }
            OverlayAction::SelectModel(id) => {
                self.overlay = None;
                if self.job.is_some() {
                    self.push_system(i18n::t(Msg::ModelBusyReject).into());
                    return SessionAction::None;
                }
                match tester_core::config::persist_llm_model(&id) {
                    Ok(path) => {
                        self.model = id.clone();
                        self.push_system(i18n::tr(
                            Msg::ModelPersisted,
                            &[("model", &id), ("path", &path.display().to_string())],
                        ));
                        self.status = i18n::tr(Msg::ModelSelected, &[("model", &id)]);
                        self.status_error = false;
                    }
                    Err(err) => {
                        self.push_system(format!("{err:#}"));
                        self.status_error = true;
                    }
                }
                SessionAction::None
            }
        }
    }

    fn open_model_overlay(&mut self, filter: Option<&str>) {
        let llm = Config::load()
            .map(|c| {
                let mut llm = c.llm;
                if llm.model.trim().is_empty() && !self.model.is_empty() {
                    llm.model = self.model.clone();
                }
                if !self.base_url.is_empty() {
                    let idx = llm
                        .providers
                        .iter()
                        .position(|p| p.id == llm.active_provider_id)
                        .unwrap_or(0);
                    if let Some(p) = llm.providers.get_mut(idx) {
                        p.base_url = self.base_url.clone();
                    }
                }
                llm.ensure_providers();
                llm
            })
            .unwrap_or_else(|_| {
                let mut llm = tester_core::config::LlmConfig {
                    model: self.model.clone(),
                    ..Default::default()
                };
                if !self.base_url.is_empty() {
                    llm.providers[0].base_url = self.base_url.clone();
                }
                llm.ensure_providers();
                llm
            });
        let mut overlay = ModelOverlay::fetch(&llm);
        if let Some(f) = filter {
            if !f.is_empty() {
                overlay.filter = f.to_string();
                overlay.selected = 0;
            }
        }
        self.overlay = Some(Overlay::Model(overlay));
    }

    fn at_needle_open(&self) -> bool {
        active_at_needle(&self.prompt.text).is_some()
    }

    fn insert_at_path(&mut self, path: &str) {
        if let Some(idx) = self.prompt.text.rfind('@') {
            self.prompt.text.truncate(idx);
            self.prompt.text.push('@');
            self.prompt.text.push_str(path);
            self.prompt.text.push(' ');
            self.prompt.cursor = self.prompt.text.len();
        }
        self.refresh_at_files();
        self.update_ghost();
    }

    fn refresh_at_files(&mut self) {
        if let Some(needle) = active_at_needle(&self.prompt.text) {
            let symbols = tester_lang::discover::discover(&self.path)
                .ok()
                .and_then(|p| tester_lang::index::index_symbols(&p).ok())
                .unwrap_or_default();
            self.at_files = list_at_candidates(&self.path, &needle, &symbols);
            if self.at_selected >= self.at_files.len() {
                self.at_selected = self.at_files.len().saturating_sub(1);
            }
        } else {
            self.at_files.clear();
            self.at_selected = 0;
        }
    }

    fn at_active(&self) -> bool {
        active_at_needle(&self.prompt.text).is_some() && !self.at_files.is_empty()
    }

    fn submit_composer(&mut self) -> SessionAction {
        let text = self.prompt.text.trim().to_string();
        if text.is_empty() {
            return SessionAction::None;
        }
        self.prompt.push_history(&text);
        self.prompt.clear();
        self.refresh_at_files();
        self.update_ghost();
        if let Some(cmd) = text.strip_prefix('/') {
            return self.handle_slash(cmd);
        }

        let specs = extract_at_specs(&text);
        let mut prompt = text.clone();
        if !specs.is_empty() {
            prompt.push_str("\n\n# Attached files\n");
            for spec in &specs {
                if let Some((path, body)) = read_attach_snippet(&self.path, spec) {
                    prompt.push_str(&format!("\n## {path}\n```\n{body}\n```\n"));
                }
            }
        }

        self.transcript.push("you", text);
        self.start_agent(prompt);
        SessionAction::None
    }

    fn handle_slash(&mut self, cmd: &str) -> SessionAction {
        let mut parts = cmd.split_whitespace();
        let name = parts.next().unwrap_or("");
        match name {
            "help" | "h" | "?" => {
                self.push_system(Self::help_text());
            }
            "exit" | "quit" | "q" => return SessionAction::Quit,
            "clear" => {
                if !self.session_id.is_empty() {
                    let _ = session_store::clear_messages(&self.session_id);
                }
                self.transcript.clear();
                self.status = i18n::t(Msg::SessionCleared).into();
                self.status_error = false;
            }
            "audit" => match open_context_with_policy(&self.path, self.dry_run, false, true) {
                Ok(ctx) => self.push_system(ctx.audit.summary()),
                Err(err) => {
                    self.push_system(i18n::tr(Msg::AuditFailed, &[("err", &format!("{err:#}"))]))
                }
            },
            "scan" => match tester_lang::discover::discover(&self.path) {
                Ok(profile) => self.push_system(i18n::tr(
                    Msg::ScanSummary,
                    &[
                        ("lang", profile.language.as_str()),
                        ("pkg", &profile.package_name),
                        ("cmd", &profile.test_command),
                    ],
                )),
                Err(err) => {
                    self.push_system(i18n::tr(Msg::ScanFailed, &[("err", &format!("{err:#}"))]))
                }
            },
            "plan" => {
                let kind_str = parts.next().unwrap_or("unit");
                match (
                    kind_str.parse::<TestKind>(),
                    open_context_with_policy(&self.path, self.dry_run, false, true),
                ) {
                    (Ok(kind), Ok(ctx)) => match build_and_save_plan(&ctx, kind) {
                        Ok(plan) => {
                            if plan.items.is_empty() {
                                self.push_system(i18n::t(Msg::SessionNoGoals).into());
                            } else {
                                let mut text = String::new();
                                for item in &plan.items {
                                    text.push_str(&format!(
                                        "- [{}] {} -> {}\n",
                                        item.kind.as_str(),
                                        item.gap.symbol.name,
                                        item.target_path.display()
                                    ));
                                }
                                self.push_system(text);
                            }
                        }
                        Err(err) => self.push_system(i18n::tr(
                            Msg::PlanFailed,
                            &[("err", &format!("{err:#}"))],
                        )),
                    },
                    (Err(err), _) => {
                        self.push_system(format!("{}: {err}", i18n::t(Msg::SessionBadKind)))
                    }
                    (_, Err(err)) => {
                        self.push_system(i18n::tr(Msg::PlanFailed, &[("err", &format!("{err:#}"))]))
                    }
                }
            }
            "propose_plan" => {
                let kind_str = parts.next().unwrap_or("unit");
                match (
                    kind_str.parse::<TestKind>(),
                    open_context_with_policy(&self.path, self.dry_run, false, true),
                ) {
                    (Ok(kind), Ok(ctx)) => match build_plan(&ctx, kind) {
                        Ok(plan) => {
                            let mut body = String::new();
                            for item in &plan.items {
                                body.push_str(&format!(
                                    "- [{}] {} -> {}\n",
                                    item.kind.as_str(),
                                    item.gap.symbol.name,
                                    item.target_path.display()
                                ));
                            }
                            if body.is_empty() {
                                self.push_system(i18n::t(Msg::SessionNoGoals).into());
                            } else {
                                self.plan_stash =
                                    i18n::tr(Msg::ProposePlanPrompt, &[("body", &body)]);
                                self.overlay = Some(Overlay::Plan(PlanOverlay::new(
                                    i18n::tr(Msg::ProposePlanTitle, &[("kind", kind_str)]),
                                    body,
                                )));
                            }
                        }
                        Err(err) => self.push_system(i18n::tr(
                            Msg::PlanFailed,
                            &[("err", &format!("{err:#}"))],
                        )),
                    },
                    (Err(err), _) => {
                        self.push_system(format!("{}: {err}", i18n::t(Msg::SessionBadKind)))
                    }
                    (_, Err(err)) => {
                        self.push_system(i18n::tr(Msg::PlanFailed, &[("err", &format!("{err:#}"))]))
                    }
                }
            }
            "run" => {
                let prompt = cmd.strip_prefix("run").unwrap_or("").trim();
                if prompt.is_empty() {
                    self.push_system(i18n::t(Msg::SessionUsageRun).into());
                } else {
                    self.transcript.push("you", prompt.to_string());
                    self.start_agent(prompt.to_string());
                }
            }
            "grep" => {
                let pattern = cmd.strip_prefix("grep").unwrap_or("").trim();
                if pattern.is_empty() {
                    self.push_system(i18n::t(Msg::UsageGrep).into());
                } else {
                    match open_context_with_policy(&self.path, true, false, true) {
                        Ok(ctx) => {
                            let args = serde_json::json!({"pattern": pattern});
                            match tester_agent::tools::default_registry(&ctx)
                                .get("grep")
                                .map(|t| t.execute(&ctx, &args))
                            {
                                Some(Ok(out)) => self.push_system(out),
                                Some(Err(e)) => self.push_system(i18n::tr(
                                    Msg::GrepFailed,
                                    &[("err", &format!("{e:#}"))],
                                )),
                                None => self
                                    .push_system(i18n::tr(Msg::ToolMissing, &[("name", "grep")])),
                            }
                        }
                        Err(e) => self
                            .push_system(i18n::tr(Msg::GrepFailed, &[("err", &format!("{e:#}"))])),
                    }
                }
            }
            "symbol" => {
                let name = cmd.strip_prefix("symbol").unwrap_or("").trim();
                if name.is_empty() {
                    self.push_system(i18n::t(Msg::UsageSymbol).into());
                } else {
                    self.run_analysis_tool("find_symbol", serde_json::json!({"name": name}));
                }
            }
            "refs" => {
                let name = cmd.strip_prefix("refs").unwrap_or("").trim();
                if name.is_empty() {
                    self.push_system(i18n::t(Msg::UsageRefs).into());
                } else {
                    self.run_analysis_tool("find_references", serde_json::json!({"name": name}));
                }
            }
            "search" => {
                let q = cmd.strip_prefix("search").unwrap_or("").trim();
                if q.is_empty() {
                    self.push_system(i18n::t(Msg::UsageSearch).into());
                } else {
                    self.run_analysis_tool("semantic_search", serde_json::json!({"query": q}));
                }
            }
            "index" => {
                let sub = parts.next().unwrap_or("status");
                match sub {
                    "rebuild" => match tester_lang::discover::discover(&self.path) {
                        Ok(profile) => match tester_lang::index::rebuild_index(&profile) {
                            Ok(syms) => self.push_system(i18n::tr(
                                Msg::IndexRebuilt,
                                &[("n", &syms.len().to_string())],
                            )),
                            Err(e) => self.push_system(i18n::tr(
                                Msg::IndexRebuildFailed,
                                &[("err", &format!("{e:#}"))],
                            )),
                        },
                        Err(e) => self
                            .push_system(i18n::tr(Msg::IndexFailed, &[("err", &format!("{e:#}"))])),
                    },
                    _ => self.run_analysis_tool("index_status", serde_json::json!({})),
                }
            }
            "map" => self.run_analysis_tool("project_map", serde_json::json!({})),
            "list_files" => self.run_analysis_tool("project_map", serde_json::json!({"depth": 5})),
            "show_file" => {
                let path = parts.next().unwrap_or("");
                if path.is_empty() {
                    self.push_system(i18n::t(Msg::UsageShowFile).into());
                } else {
                    self.run_analysis_tool("outline_file", serde_json::json!({"path": path}));
                }
            }
            "manual" => self.overlay = Some(Overlay::Manual(ManualOverlay::new())),
            "recent" => {
                self.overlay = Some(Overlay::Recent(SessionListOverlay::new(Some(
                    self.path.clone(),
                ))))
            }
            "session" | "sessions" => {
                let arg = parts.next().unwrap_or("");
                match arg {
                    "" => {
                        self.overlay = Some(Overlay::ChatSessions(ChatSessionsOverlay::new(
                            self.path.clone(),
                        )));
                    }
                    "new" => self.switch_chat_session(None),
                    id => match session_store::resolve_id(id) {
                        Ok(id) => self.switch_chat_session(Some(id)),
                        Err(e) => self.push_system(i18n::tr(
                            Msg::SessionCmdError,
                            &[("err", &format!("{e:#}"))],
                        )),
                    },
                }
            }
            "prune" => {
                let arg = parts.next().unwrap_or("");
                let spec = match arg {
                    "" | "help" => {
                        self.push_system(i18n::t(Msg::PruneUsage).into());
                        return SessionAction::None;
                    }
                    "all" => PruneSpec::All,
                    "-w" => PruneSpec::Workspace(self.path.clone()),
                    "-o" => {
                        let days = parts
                            .next()
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(30);
                        PruneSpec::OlderThanDays(days)
                    }
                    id => PruneSpec::Id(id.to_string()),
                };
                match session_store::prune(spec) {
                    Ok(n) => {
                        self.push_system(i18n::tr(Msg::PruneDone, &[("n", &n.to_string())]));

                        if !self.session_id.is_empty()
                            && session_store::read_meta(&self.session_id).is_err()
                        {
                            self.switch_chat_session(None);
                        }
                    }
                    Err(e) => {
                        self.push_system(i18n::tr(Msg::PruneError, &[("err", &format!("{e:#}"))]))
                    }
                }
            }
            "diff" => self.overlay = Some(Overlay::Diff(DiffOverlay::from_git(&self.path))),
            "security" => {
                let deep = parts.next() == Some("deep");
                self.overlay = Some(Overlay::Security(SecurityOverlay::scan(&self.path, deep)));
            }
            "branch" => self.overlay = Some(Overlay::Branch(BranchOverlay::from_git(&self.path))),
            "model" => {
                let filter = cmd.strip_prefix("model").unwrap_or("").trim();
                self.open_model_overlay(if filter.is_empty() {
                    None
                } else {
                    Some(filter)
                });
            }
            other => self.push_system(format!("/{other}: {}", i18n::t(Msg::SessionUnknownCmd))),
        }
        SessionAction::None
    }

    fn switch_chat_session(&mut self, id: Option<String>) {
        let loaded = match id {
            Some(id) => session_store::resolve_id(&id).and_then(|id| {
                let (meta, session) = session_store::load(&id)?;
                session_store::set_active(&self.path, &meta.id)?;
                Ok((meta, session))
            }),
            None => session_store::create(&self.path),
        };
        match loaded {
            Ok((meta, session)) => {
                self.session_id = meta.id.clone();
                self.prompt.clear();
                self.transcript.clear();
                self.hydrate_from_session(&session);
                self.status = format!(
                    "{}{}",
                    &meta.id[..meta.id.len().min(8)],
                    i18n::t(Msg::SessionSwitched)
                );
                self.status_error = false;
                self.update_ghost();
            }
            Err(e) => {
                self.push_system(i18n::tr(
                    Msg::SessionSwitchFailed,
                    &[("err", &format!("{e:#}"))],
                ));
                self.status_error = true;
            }
        }
    }

    fn require_model(&mut self) -> bool {
        if self.model.trim().is_empty() {
            self.push_system(i18n::t(Msg::ModelSelectHint).into());
            self.status = i18n::t(Msg::ModelSelectHint).into();
            self.status_error = true;
            self.open_model_overlay(None);
            false
        } else {
            true
        }
    }

    fn run_analysis_tool(&mut self, name: &str, args: serde_json::Value) {
        if !self.require_model() {
            return;
        }
        let model = self.model.clone();
        match open_context_ex(&self.path, true, false, true, Some(&model)) {
            Ok(ctx) => match tester_agent::tools::default_registry(&ctx)
                .get(name)
                .map(|t| t.execute(&ctx, &args))
            {
                Some(Ok(out)) => {
                    let preview = if out.len() > 4000 {
                        format!("{}...", &out[..4000])
                    } else {
                        out
                    };
                    self.push_system(preview);
                }
                Some(Err(e)) => self.push_system(i18n::tr(
                    Msg::ToolFailed,
                    &[("name", name), ("err", &format!("{e:#}"))],
                )),
                None => self.push_system(i18n::tr(Msg::ToolMissing, &[("name", name)])),
            },
            Err(e) => self.push_system(i18n::tr(
                Msg::ToolFailed,
                &[("name", name), ("err", &format!("{e:#}"))],
            )),
        }
    }

    fn start_agent(&mut self, prompt: String) {
        if !self.require_model() {
            return;
        }
        let path = self.path.clone();
        let model = self.model.clone();
        let dry_run = self.dry_run;
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_job = Arc::clone(&cancel);
        let stream_buf = Arc::new(Mutex::new(String::new()));
        let stream_job = Arc::clone(&stream_buf);
        let (tx, rx) = mpsc::channel();
        let (perm_tx, perm_rx) = mpsc::sync_channel::<PermReq>(0);
        self.job = Some(Job {
            rx,
            cancel,
            stream_buf,
            perm_rx,
            started: Instant::now(),
        });
        self.streaming_idx = None;
        self.status = i18n::t(Msg::SessionRunning).into();
        self.status_error = false;
        thread::spawn(move || {
            let result = (|| {
                let mut ctx = open_context_ex(&path, dry_run, true, true, Some(&model))?;
                ctx.cancel = cancel_job;
                if !ctx.yes && !dry_run {
                    let perm_tx = perm_tx;
                    ctx.permissions = PermissionPolicy::for_tui(false, false).with_confirm(
                        Arc::new(move |tool: &str, preview: &str| {
                            let (reply_tx, reply_rx) = mpsc::sync_channel(0);
                            if perm_tx
                                .send(PermReq {
                                    tool: tool.to_string(),
                                    preview: preview.to_string(),
                                    reply: reply_tx,
                                })
                                .is_err()
                            {
                                return false;
                            }
                            reply_rx.recv().unwrap_or(false)
                        }),
                    );
                }
                let on_delta = {
                    let buf = Arc::clone(&stream_job);
                    Arc::new(move |delta: &str| {
                        if let Ok(mut g) = buf.lock() {
                            g.push_str(delta);
                        }
                    }) as Arc<dyn Fn(&str) + Send + Sync>
                };
                run_one_shot_ex(
                    &ctx,
                    &prompt,
                    RunOneShotOpts {
                        resume: true,
                        on_delta: Some(on_delta),
                    },
                )
            })()
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send(result);
        });
    }

    fn push_system(&mut self, text: String) {
        self.transcript.push("system", text);
    }

    fn help_text() -> String {
        let mut out = String::from(i18n::t(Msg::SessionHelp));
        out.push('\n');
        for cmd in SLASH_CMDS {
            out.push_str(&format!("  /{cmd:<14} {}\n", slash_desc(cmd)));
        }
        out
    }

    fn slash_active(&self) -> bool {
        self.prompt.text.starts_with('/') && !self.prompt.text[1..].contains(' ')
    }

    fn slash_prefix(&self) -> &str {
        self.prompt.text.strip_prefix('/').unwrap_or("")
    }

    fn slash_filtered(&self) -> Vec<&'static str> {
        let prefix = self.slash_prefix().to_lowercase();
        SLASH_CMDS
            .iter()
            .copied()
            .filter(|c| prefix.is_empty() || c.starts_with(&prefix))
            .collect()
    }

    fn update_ghost(&mut self) {
        self.prompt.ghost.clear();
        if !self.slash_active() {
            return;
        }
        let filtered = self.slash_filtered();
        if filtered.is_empty() {
            self.slash_selected = 0;
            return;
        }
        if self.slash_selected >= filtered.len() {
            self.slash_selected = filtered.len() - 1;
        }
        if let Some(cmd) = filtered.get(self.slash_selected) {
            self.prompt.ghost = format!("/{cmd}");
        }
    }

    fn slash_popup_items(&self) -> Vec<PopupItem> {
        self.slash_filtered()
            .iter()
            .map(|cmd| PopupItem::new(format!("/{cmd}"), slash_desc(cmd)))
            .collect()
    }

    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        if let Some(overlay) = &self.overlay {
            overlay.draw(frame, area, theme);
            if let Some(req) = &self.pending_perm {
                self.draw_perm_modal(frame, area, theme, req);
            }
            return;
        }

        let narrow = area.width < 60 || area.height < 12;
        let very_narrow = area.width < 40;
        let composer_h: u16 = if narrow { 3 } else { 4 };
        let status_h: u16 = 2;

        let room_above = area
            .height
            .saturating_sub(composer_h.saturating_add(status_h))
            .max(2);
        let palette_h = if self.slash_active() {
            let n = self.slash_filtered().len().max(1);
            let max_h = if very_narrow {
                5
            } else if narrow {
                7
            } else {
                12
            };
            popup_height(n, max_h.min(room_above))
        } else if self.at_needle_open() {
            let n = self.at_files.len().max(1);
            let max_h = if very_narrow {
                4
            } else if narrow {
                6
            } else {
                10
            };
            popup_height(n.min(10), max_h.min(room_above))
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(composer_h),
                Constraint::Length(status_h),
            ])
            .split(area);

        self.transcript
            .draw(frame, chunks[0], theme, self.streaming_idx);

        let composer_block = Block::default()
            .borders(Borders::TOP)
            .border_style(theme.faint())
            .style(theme.base());
        let composer_inner = composer_block.inner(chunks[1]);
        frame.render_widget(composer_block, chunks[1]);
        frame.render_widget(
            self.prompt
                .display_lines(theme, composer_inner.width)
                .style(theme.base()),
            composer_inner,
        );

        self.draw_status_bar(frame, chunks[2], theme, area.width);

        if palette_h > 0 {
            let popup_area = place_above(chunks[0], chunks[1].y, palette_h);
            if self.at_needle_open() {
                let items: Vec<PopupItem> = if self.at_files.is_empty() {
                    vec![PopupItem::new(i18n::t(Msg::PopupNoMatches), "")]
                } else {
                    self.at_files
                        .iter()
                        .map(|f| PopupItem::new(format!("@{f}"), i18n::t(Msg::AttachHint)))
                        .collect()
                };
                draw_popup(
                    frame,
                    popup_area,
                    theme,
                    &Popup {
                        title: i18n::t(Msg::TitleFiles),
                        items: &items,
                        selected: self.at_selected,
                        show_descriptions: !narrow && !very_narrow,
                    },
                );
            } else if self.slash_active() {
                let items = self.slash_popup_items();
                draw_popup(
                    frame,
                    popup_area,
                    theme,
                    &Popup {
                        title: i18n::t(Msg::TitleCommands),
                        items: &items,
                        selected: self.slash_selected,
                        show_descriptions: !narrow && !very_narrow,
                    },
                );
            }
        }

        if let Some(req) = &self.pending_perm {
            self.draw_perm_modal(frame, area, theme, req);
        }
    }

    fn draw_status_bar(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme, width: u16) {
        let (used, limit) = self.ctx_gauge();
        let phase = {
            let b = self.badges();
            if b.is_empty() {
                i18n::t(Msg::StatusIdle).to_string()
            } else {
                b
            }
        };
        let phase_style = if self.job.is_some() {
            theme.accent_style()
        } else if self.pending_perm.is_some() {
            theme.warning_style()
        } else {
            theme.muted()
        };

        let mut top = vec![
            Span::styled(format!(" {phase}"), phase_style),
            Span::styled("", theme.faint()),
            Span::styled(
                i18n::tr(
                    Msg::CtxGauge,
                    &[("used", &used.to_string()), ("limit", &limit.to_string())],
                ),
                theme.faint(),
            ),
        ];
        if self.pending_perm.is_some() {
            top.push(Span::styled(
                i18n::t(Msg::PermWaitYn).to_string(),
                theme.warning_style(),
            ));
        }

        let detail = if self.status.is_empty() {
            i18n::t(Msg::StatusHelpHint).to_string()
        } else {
            truncate_middle(&self.status, width.saturating_sub(2) as usize)
        };
        let detail_style = if self.status_error {
            theme.error_style()
        } else {
            theme.muted()
        };

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(top),
                Line::from(Span::styled(format!(" {detail}"), detail_style)),
            ])
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(theme.faint()),
            )
            .style(theme.base()),
            area,
        );
    }

    fn draw_perm_modal(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme, req: &PermReq) {
        let modal = centered(area, 62, 10);
        frame.render_widget(Clear, modal);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_block(true))
            .title(Span::styled(
                i18n::t(Msg::PermTitle),
                theme.warning_style().add_modifier(Modifier::BOLD),
            ))
            .style(theme.raised());
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let preview: String = req.preview.chars().take(180).collect();
        let body = vec![
            Line::from(Span::styled(
                req.tool.clone(),
                theme.accent_style().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(preview, theme.muted())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  y ", theme.success_style().add_modifier(Modifier::BOLD)),
                Span::styled(i18n::t(Msg::PermAllow), theme.raised()),
                Span::styled("    ", theme.raised()),
                Span::styled(" n ", theme.error_style().add_modifier(Modifier::BOLD)),
                Span::styled(i18n::t(Msg::PermDeny), theme.raised()),
            ]),
            Line::from(Span::styled(
                format!("  {}", i18n::t(Msg::PermPrompt)),
                theme.faint(),
            )),
        ];
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: true })
                .style(theme.raised()),
            inner,
        );
    }
}

fn centered(area: Rect, percent_x: u16, height: u16) -> Rect {
    let w = area.width.saturating_mul(percent_x) / 100;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: w.max(24),
        height,
    }
}

fn is_prompt_newline_key(key: &KeyEvent) -> bool {
    let mods = key.modifiers;
    let with_mod = mods.contains(KeyModifiers::SHIFT)
        || mods.contains(KeyModifiers::ALT)
        || mods.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r') => with_mod,
        KeyCode::Char('j') | KeyCode::Char('J') if mods.contains(KeyModifiers::CONTROL) => true,
        _ => false,
    }
}

#[cfg(test)]
mod slash_tests {
    use super::SLASH_CMDS;

    #[test]
    fn slash_filter_prefix() {
        let filtered: Vec<_> = SLASH_CMDS
            .iter()
            .copied()
            .filter(|c| c.starts_with("pl"))
            .collect();
        assert_eq!(filtered, vec!["plan"]);
    }
}
