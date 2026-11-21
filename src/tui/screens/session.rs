use crate::agent::{
    clear_session, load_session, open_context_with_policy, run_one_shot_ex, save_session,
    AgentOutcome, AgentSession, PermissionPolicy, RunOneShotOpts,
};
use crate::i18n::{self, Msg};
use crate::model::TestKind;
use crate::plan::{PlanOptions, build_plan_with};
use crate::plan_store::save_plan;
use crate::tui::files::{
    active_at_needle, extract_at_specs, list_project_files, read_attach_snippet,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread;

const SLASH_CMDS: &[&str] = &[
    "help", "scan", "plan", "run", "config", "clear", "audit", "exit",
];

#[derive(Debug, Clone)]
pub enum SessionAction {
    None,
    Quit,
    OpenPicker,
}

#[derive(Debug, Clone)]
struct LineEntry {
    role: &'static str,
    text: String,
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
}

pub struct SessionScreen {
    path: PathBuf,
    model: String,
    base_url: String,
    dry_run: bool,
    auto_approve: bool,
    lines: Vec<LineEntry>,
    composer: String,
    status: String,
    job: Option<Job>,
    slash_selected: usize,
    at_selected: usize,
    at_files: Vec<String>,
    attachments: Vec<String>,
    pending_perm: Option<PermReq>,
    streaming_idx: Option<usize>,
}

impl SessionScreen {
    pub fn root(&self) -> &std::path::Path {
        &self.path
    }

    pub fn open(path: PathBuf, dry_run: bool, resume: bool) -> Self {
        let mut screen = Self {
            path: path.clone(),
            model: String::new(),
            base_url: String::new(),
            dry_run,
            auto_approve: false,
            lines: Vec::new(),
            composer: String::new(),
            status: String::new(),
            job: None,
            slash_selected: 0,
            at_selected: 0,
            at_files: Vec::new(),
            attachments: Vec::new(),
            pending_perm: None,
            streaming_idx: None,
        };
        match open_context_with_policy(&path, dry_run, false, true) {
            Ok(ctx) => {
                screen.model = ctx.config.llm.model.clone();
                screen.base_url = ctx.config.llm.base_url.clone();
                screen.auto_approve = ctx.config.run.agent_auto_approve;
                screen.path = ctx.profile.root.clone();

                let _ = resume;
                if let Ok(session) = load_session(&screen.path) {
                    screen.hydrate_from_session(&session);
                }
                screen.status = if dry_run {
                    i18n::t(Msg::SessionStatusDryRun).into()
                } else if screen.auto_approve {
                    i18n::t(Msg::SessionStatusAuto).into()
                } else {
                    i18n::t(Msg::SessionStatusBlocked).into()
                };
            }
            Err(err) => {
                screen.lines.push(LineEntry {
                    role: "system",
                    text: format!("{}: {err:#}", i18n::t(Msg::SessionFailedOpen)),
                });
                screen.status = i18n::t(Msg::SessionFixHint).into();
            }
        }
        screen
    }

    fn hydrate_from_session(&mut self, session: &AgentSession) {
        self.lines.clear();
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
                    body.push_str(&format!(
                        "-> tool {}({})",
                        c.function.name, c.function.arguments
                    ));
                }
            }
            if body.trim().is_empty() {
                continue;
            }
            self.lines.push(LineEntry { role, text: body });
        }
    }

    pub fn tick(&mut self) {
        if self.pending_perm.is_none() {
            if let Some(job) = &self.job {
                if let Ok(req) = job.perm_rx.try_recv() {
                    self.pending_perm = Some(req);
                    self.status = i18n::t(Msg::PermPrompt).into();
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
                    if let Some(line) = self.lines.get_mut(idx) {
                        line.text = text;
                    }
                } else {
                    self.lines.push(LineEntry {
                        role: "assistant",
                        text,
                    });
                    self.streaming_idx = Some(self.lines.len() - 1);
                }
            }
        }

        match job.rx.try_recv() {
            Ok(Ok(outcome)) => {
                self.job = None;
                self.pending_perm = None;
                let text = if outcome.final_text.trim().is_empty() {
                    i18n::t(Msg::SessionEmptyResponse).into()
                } else {
                    outcome.final_text
                };
                if let Some(idx) = self.streaming_idx.take() {
                    if let Some(line) = self.lines.get_mut(idx) {
                        line.text = text;
                    }
                } else {
                    self.lines.push(LineEntry {
                        role: "assistant",
                        text,
                    });
                }
                self.status = format!(
                    "turns={} tools={} tokens≈{} {}",
                    outcome.turns,
                    outcome.tool_calls,
                    outcome.usage.total(),
                    if outcome.cancelled {
                        "cancelled"
                    } else {
                        "ok"
                    }
                );
            }
            Ok(Err(err)) => {
                self.job = None;
                self.pending_perm = None;
                self.streaming_idx = None;
                self.lines.push(LineEntry {
                    role: "system",
                    text: format!("{}{err}", i18n::t(Msg::ErrorPrefix)),
                });
                self.status = i18n::t(Msg::SessionError).into();
            }
            Err(TryRecvError::Empty) => {
                if self.pending_perm.is_none() {
                    self.status = i18n::t(Msg::SessionRunning).into();
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.pending_perm = None;
                self.status = i18n::t(Msg::SessionThreadExit).into();
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

        if self.job.is_some() {
            if key.code == KeyCode::Esc
                || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
            {
                if let Some(job) = &self.job {
                    job.cancel.store(true, Ordering::SeqCst);
                }
                self.status = i18n::t(Msg::SessionCancelReq).into();
            }
            return SessionAction::None;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
            return SessionAction::OpenPicker;
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
                KeyCode::Esc => {
                    if let Some(idx) = self.composer.rfind('@') {
                        self.composer.truncate(idx);
                        self.refresh_at_files();
                    }
                    return SessionAction::None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                if self.composer.is_empty() {
                    return SessionAction::Quit;
                }
                self.composer.clear();
                self.refresh_at_files();
            }
            KeyCode::Up if self.slash_active() => {
                self.slash_selected = self.slash_selected.saturating_sub(1);
            }
            KeyCode::Down if self.slash_active() => {
                let n = self.slash_filtered().len();
                if n > 0 {
                    self.slash_selected = (self.slash_selected + 1).min(n - 1);
                }
            }
            KeyCode::Tab if self.slash_active() => {
                if let Some(cmd) = self.slash_filtered().get(self.slash_selected).copied() {
                    self.composer = format!("/{cmd} ");
                    self.slash_selected = 0;
                }
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.composer.push('\n');
                    return SessionAction::None;
                }
                if self.slash_active() {
                    let filtered = self.slash_filtered();
                    let exact = SLASH_CMDS
                        .iter()
                        .any(|c| self.composer.trim() == format!("/{c}"));
                    if !exact {
                        if let Some(cmd) = filtered.get(self.slash_selected).copied() {
                            if filtered.len() == 1 || self.composer.trim() == "/" {
                                self.composer = format!("/{cmd}");
                            } else if !self.composer.contains(' ') {
                                self.composer = format!("/{cmd} ");
                                return SessionAction::None;
                            }
                        }
                    }
                }
                return self.submit_composer();
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.composer.push('\n');
            }
            KeyCode::Backspace => {
                self.composer.pop();
                self.slash_selected = 0;
                self.refresh_at_files();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !key.modifiers.contains(KeyModifiers::SUPER) =>
            {
                self.composer.push(c);
                self.slash_selected = 0;
                self.refresh_at_files();
            }
            _ => {}
        }
        SessionAction::None
    }

    fn insert_at_path(&mut self, path: &str) {
        if let Some(idx) = self.composer.rfind('@') {
            self.composer.truncate(idx);
            self.composer.push('@');
            self.composer.push_str(path);
            self.composer.push(' ');
        }
        if !self.attachments.iter().any(|a| a == path) {
            self.attachments.push(path.to_string());
        }
        self.refresh_at_files();
    }

    fn refresh_at_files(&mut self) {
        if let Some(needle) = active_at_needle(&self.composer) {
            self.at_files = list_project_files(&self.path, &needle);
            if self.at_selected >= self.at_files.len() {
                self.at_selected = self.at_files.len().saturating_sub(1);
            }
        } else {
            self.at_files.clear();
            self.at_selected = 0;
        }
    }

    fn at_active(&self) -> bool {
        active_at_needle(&self.composer).is_some() && !self.at_files.is_empty()
    }

    fn submit_composer(&mut self) -> SessionAction {
        let text = self.composer.trim().to_string();
        if text.is_empty() {
            return SessionAction::None;
        }
        self.composer.clear();
        self.refresh_at_files();
        if let Some(cmd) = text.strip_prefix('/') {
            return self.handle_slash(cmd);
        }

        for spec in extract_at_specs(&text) {
            let path = spec.split('#').next().unwrap_or(&spec).to_string();
            if !self.attachments.iter().any(|a| a == &path) {
                self.attachments.push(path);
            }
        }

        let mut prompt = text.clone();
        if !self.attachments.is_empty() {
            prompt.push_str("\n\n# Attached files\n");
            for spec in extract_at_specs(&text) {
                if let Some((path, body)) = read_attach_snippet(&self.path, &spec) {
                    prompt.push_str(&format!("\n## {path}\n```\n{body}\n```\n"));
                }
            }
        }

        self.lines.push(LineEntry {
            role: "you",
            text,
        });
        self.start_agent(prompt);
        SessionAction::None
    }

    fn handle_slash(&mut self, cmd: &str) -> SessionAction {
        let mut parts = cmd.split_whitespace();
        let name = parts.next().unwrap_or("");
        match name {
            "help" | "h" | "?" => {
                self.push_system(i18n::t(Msg::SessionHelp).into());
            }
            "exit" | "quit" | "q" => return SessionAction::Quit,
            "clear" => {
                let _ = clear_session(&self.path);
                let _ = save_session(&self.path, &AgentSession::new());
                self.lines.clear();
                self.attachments.clear();
                self.status = i18n::t(Msg::SessionCleared).into();
            }
            "config" => {
                self.push_system(format!(
                    "model={} base_url={} dry_run={} agent_auto_approve={}",
                    self.model, self.base_url, self.dry_run, self.auto_approve
                ));
            }
            "audit" => match open_context_with_policy(&self.path, self.dry_run, false, true) {
                Ok(ctx) => self.push_system(ctx.audit.summary()),
                Err(err) => self.push_system(format!("audit failed: {err:#}")),
            },
            "scan" => match crate::discover::discover(&self.path) {
                Ok(profile) => self.push_system(format!(
                    "language={} package={} cmd={}",
                    profile.language.as_str(),
                    profile.package_name,
                    profile.test_command
                )),
                Err(err) => self.push_system(format!("scan failed: {err:#}")),
            },
            "plan" => {
                let kind_str = parts.next().unwrap_or("unit");
                match (
                    kind_str.parse::<TestKind>(),
                    open_context_with_policy(&self.path, self.dry_run, false, true),
                ) {
                    (Ok(kind), Ok(ctx)) => match build_plan_with(
                        &ctx.profile,
                        &ctx.config,
                        kind,
                        &PlanOptions::default(),
                    ) {
                        Ok(plan) => {
                            let _ = save_plan(&ctx.profile.root, &plan);
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
                        Err(err) => self.push_system(format!("plan failed: {err:#}")),
                    },
                    (Err(err), _) => {
                        self.push_system(format!("{}: {err}", i18n::t(Msg::SessionBadKind)))
                    }
                    (_, Err(err)) => self.push_system(format!("plan failed: {err:#}")),
                }
            }
            "run" => {
                let prompt = cmd.strip_prefix("run").unwrap_or("").trim();
                if prompt.is_empty() {
                    self.push_system(i18n::t(Msg::SessionUsageRun).into());
                } else {
                    self.lines.push(LineEntry {
                        role: "you",
                        text: prompt.to_string(),
                    });
                    self.start_agent(prompt.to_string());
                }
            }
            other => self.push_system(format!("/{other}: {}", i18n::t(Msg::SessionUnknownCmd))),
        }
        SessionAction::None
    }

    fn start_agent(&mut self, prompt: String) {
        let path = self.path.clone();
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
        });
        self.streaming_idx = None;
        self.status = i18n::t(Msg::SessionRunning).into();
        thread::spawn(move || {
            let result = (|| {
                let mut ctx = open_context_with_policy(&path, dry_run, true, true)?;
                ctx.cancel = cancel_job;
                if !ctx.yes && !dry_run {
                    let perm_tx = perm_tx;
                    ctx.permissions = PermissionPolicy::for_tui(false, false).with_confirm(Arc::new(
                        move |tool: &str, preview: &str| {
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
                        },
                    ));
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
        self.lines.push(LineEntry {
            role: "system",
            text,
        });
    }

    fn slash_active(&self) -> bool {
        self.composer.starts_with('/') && !self.composer[1..].contains(' ')
    }

    fn slash_prefix(&self) -> &str {
        self.composer.strip_prefix('/').unwrap_or("")
    }

    fn slash_filtered(&self) -> Vec<&'static str> {
        let prefix = self.slash_prefix().to_lowercase();
        SLASH_CMDS
            .iter()
            .copied()
            .filter(|c| prefix.is_empty() || c.starts_with(&prefix))
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        let palette_h = if self.at_active() || self.slash_active() {
            7
        } else {
            0
        };
        let chip_h = if self.attachments.is_empty() { 0 } else { 3 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(chip_h),
                Constraint::Length(5),
                Constraint::Length(palette_h),
            ])
            .split(area);

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                i18n::t(Msg::TitleSession),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(self.path.display().to_string()),
            Span::raw(format!("    {}", self.model)),
            if self.dry_run {
                Span::raw("   dry-run")
            } else {
                Span::raw("")
            },
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = self
            .lines
            .iter()
            .map(|l| {
                let style = match l.role {
                    "you" => Style::default().add_modifier(Modifier::BOLD),
                    "system" => Style::default().add_modifier(Modifier::ITALIC),
                    _ => Style::default(),
                };
                let role = match l.role {
                    "you" => i18n::t(Msg::RoleYou),
                    "assistant" => i18n::t(Msg::RoleAssistant),
                    "tool" => i18n::t(Msg::RoleTool),
                    _ => i18n::t(Msg::RoleSystem),
                };
                ListItem::new(format!("[{role}] {}", l.text)).style(style)
            })
            .collect();
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(i18n::t(Msg::TitleTranscript)),
        );
        frame.render_widget(list, chunks[1]);

        if chip_h > 0 {
            let chips = self
                .attachments
                .iter()
                .map(|a| format!("@{a}"))
                .collect::<Vec<_>>()
                .join("  ");
            let chip = Paragraph::new(chips).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(i18n::t(Msg::TitleAttach)),
            );
            frame.render_widget(chip, chunks[2]);
        }

        let composer = Paragraph::new(self.composer.as_str())
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(format!(
                "{}  {}",
                i18n::t(Msg::TitleComposer).trim(),
                self.status
            )));
        frame.render_widget(composer, chunks[3]);

        if palette_h > 0 {
            if self.at_active() {
                let items: Vec<ListItem> = self
                    .at_files
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let mark = if i == self.at_selected { ">" } else { " " };
                        ListItem::new(format!("{mark} @{f}"))
                    })
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n::t(Msg::TitleFiles)),
                );
                frame.render_widget(list, chunks[4]);
            } else if self.slash_active() {
                let filtered = self.slash_filtered();
                let items: Vec<ListItem> = filtered
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mark = if i == self.slash_selected { ">" } else { " " };
                        ListItem::new(format!("{mark} /{c}"))
                    })
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n::t(Msg::TitleCommands)),
                );
                frame.render_widget(list, chunks[4]);
            }
        }

        if let Some(req) = &self.pending_perm {
            let area = centered(frame.area(), 60, 8);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(i18n::t(Msg::PermTitle));
            let body = Paragraph::new(format!(
                "{}\n{}\n\n{}",
                req.tool,
                req.preview.chars().take(160).collect::<String>(),
                i18n::t(Msg::PermHint)
            ))
            .block(block)
            .wrap(Wrap { trim: true });
            frame.render_widget(Clear, area);
            frame.render_widget(body, area);
        }
    }
}

fn centered(area: Rect, percent_x: u16, height: u16) -> Rect {
    let w = area.width.saturating_mul(percent_x) / 100;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: w.max(20),
        height,
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
