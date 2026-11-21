use crate::i18n::{self, Msg};
use crate::agent::{
    clear_session, load_session, open_context_with_policy, run_one_shot, save_session,
    AgentOutcome, AgentSession,
};
use crate::model::TestKind;
use crate::plan::{PlanOptions, build_plan_with};
use crate::plan_store::save_plan;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
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

struct Job {
    rx: Receiver<std::result::Result<AgentOutcome, String>>,
    cancel: Arc<AtomicBool>,
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
        let Some(job) = &self.job else {
            return;
        };
        match job.rx.try_recv() {
            Ok(Ok(outcome)) => {
                self.job = None;
                self.lines.push(LineEntry {
                    role: "assistant",
                    text: if outcome.final_text.trim().is_empty() {
                        i18n::t(Msg::SessionEmptyResponse).into()
                    } else {
                        outcome.final_text
                    },
                });
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
                self.lines.push(LineEntry {
                    role: "system",
                    text: format!("{}{err}", i18n::t(Msg::ErrorPrefix)),
                });
                self.status = i18n::t(Msg::SessionError).into();
            }
            Err(TryRecvError::Empty) => {
                self.status = i18n::t(Msg::SessionRunning).into();
            }
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                self.status = i18n::t(Msg::SessionThreadExit).into();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SessionAction {
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

        match key.code {
            KeyCode::Esc => {
                if self.composer.is_empty() {
                    return SessionAction::Quit;
                }
                self.composer.clear();
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
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !key.modifiers.contains(KeyModifiers::SUPER) =>
            {
                self.composer.push(c);
                self.slash_selected = 0;
            }
            _ => {}
        }
        SessionAction::None
    }

    fn submit_composer(&mut self) -> SessionAction {
        let text = self.composer.trim().to_string();
        if text.is_empty() {
            return SessionAction::None;
        }
        self.composer.clear();
        if let Some(cmd) = text.strip_prefix('/') {
            return self.handle_slash(cmd);
        }
        self.lines.push(LineEntry {
            role: "you",
            text: text.clone(),
        });
        self.start_agent(text);
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
        let (tx, rx) = mpsc::channel();
        self.job = Some(Job { rx, cancel });
        self.status = i18n::t(Msg::SessionRunning).into();
        thread::spawn(move || {
            let result = (|| {
                let mut ctx = open_context_with_policy(&path, dry_run, false, true)?;
                ctx.cancel = cancel_job;
                run_one_shot(&ctx, &prompt, true)
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
        let slash_h = if self.slash_active() { 6 } else { 0 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(5),
                Constraint::Length(slash_h),
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

        let composer = Paragraph::new(self.composer.as_str())
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(format!(
                "{}  {}",
                i18n::t(Msg::TitleComposer).trim(),
                self.status
            )));
        frame.render_widget(composer, chunks[2]);

        if self.slash_active() && chunks[3].height > 0 {
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
            frame.render_widget(list, chunks[3]);
        }
    }
}
