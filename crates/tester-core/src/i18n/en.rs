use super::Msg;

pub(crate) fn t(msg: Msg) -> &'static str {
    match msg {
        Msg::ErrorPrefix => "error: ",
        Msg::CancelHint => "\n^C - cancel after the current step...",
        Msg::NoGoals => "no goals",
        Msg::WarningPrefix => "warning: ",
        Msg::Language => "language",
        Msg::Package => "package",
        Msg::Supported => "supported",
        Msg::TestCommand => "test_command",
        Msg::TestStyle => "test_style",
        Msg::HasTests => "has_tests",
        Msg::UnitFramework => "unit_framework",
        Msg::UiFramework => "ui_framework",
        Msg::UiReady => "ui_ready",
        Msg::Capabilities => "capabilities",
        Msg::Note => "note",
        Msg::PlanArrow => "->",
        Msg::LlmUnavailable => "llm unavailable",
        Msg::UnsupportedLanguage => "language recognized but not supported yet",
        Msg::PlaywrightInit => "Playwright initialized in",
        Msg::ConfigLabel => "config",
        Msg::TestsLabel => "tests",

        Msg::TuiNeedsTty => {
            "tester requires an interactive terminal; use `tester chat <PATH>` / `--plain`, or run in a TTY"
        }
        Msg::TitleCommands => " /commands↑↓TabEnter ",
        Msg::TitleOpenProject => " open project ",
        Msg::TitleFiles => " @files ",
        Msg::TitleWorktrees => " worktrees ",
        Msg::PopupNoMatches => "no matches",
        Msg::AttachHint => "attach file or symbol",
        Msg::SlashHelp => "commands and keybinds",
        Msg::SlashScan => "discover project language/tests",
        Msg::SlashPlan => "build test plan (unit|e2e|...)",
        Msg::SlashRun => "run agent with prompt",
        Msg::SlashClear => "clear chat session",
        Msg::SlashAudit => "audit capability summary",
        Msg::SlashExit => "quit session",
        Msg::SlashGrep => "regex search in repo",
        Msg::SlashSymbol => "find symbol definition",
        Msg::SlashRefs => "find symbol references",
        Msg::SlashSearch => "semantic / natural search",
        Msg::SlashIndex => "index status or rebuild",
        Msg::SlashMap => "project structure map",
        Msg::SlashListFiles => "list project files",
        Msg::SlashShowFile => "outline a single file",
        Msg::SlashManual => "open manual overlay",
        Msg::SlashRecent => "recent projects picker",
        Msg::SlashSession => "list / switch / new chat session",
        Msg::SlashPrune => "delete stored sessions",
        Msg::SlashDiff => "git diff viewer",
        Msg::SlashBranch => "git branch checkout",
        Msg::SlashProposePlan => "preview plan before run",
        Msg::PickerStatusProject => "project root detected - Ctrl+Enter / Ctrl+S to open",
        Msg::PickerStatusNav => {
            "↑↓ navigate  Enter open folder  -> worktrees  Ctrl+Enter select  Esc cancel"
        }
        Msg::PickerStatusWorktrees => {
            "worktrees - Enter/Ctrl+Enter open  <- back to folders  Esc cancel"
        }
        Msg::PickerFilter => "filter",
        Msg::PickerProjectTag => "[project]",
        Msg::PickerWorktreeTag => "[worktree]",
        Msg::SessionFailedOpen => "failed to open project",
        Msg::SessionFixHint => "fix project / config, then Ctrl+O",
        Msg::SessionStatusDryRun => "dry-run  Enter send  @file  /help",
        Msg::SessionStatusAuto => "auto-approve  Enter send  @file  /help",
        Msg::SessionStatusBlocked => "write/shell need y/N confirm  Enter send  @file  /help",
        Msg::SessionCleared => "session cleared",
        Msg::SessionSwitched => "session ready",
        Msg::SessionRunning => "running... Esc to cancel",
        Msg::SessionCancelReq => "cancel requested...",
        Msg::SessionHelp => {
            "\
Tester help

Keys
  Enter            send message
  Shift+Enter / Ctrl+Enter / Ctrl+J   new line in prompt
  Esc Esc          cancel / close overlay
  Ctrl+C Ctrl+C    quit
  Ctrl+O           open project
  Ctrl+R           search prompt history
  Ctrl+X Ctrl+E    external editor
  ↑↓ / PgUp PgDn   scroll transcript
  y / Enter        allow tool
  n / Esc          deny tool

Prompt editing
  Ctrl+A / E       line start / end
  Ctrl+Home / End  buffer start / end
  Ctrl+W / U / K   kill word / line<- / line->
  Ctrl+Y           yank
  Ctrl+P / N       prompt history
  Ctrl+<- / ->       word jump
  Ctrl+V           paste clipboard
  Shift+Insert     paste
  mouse            native terminal select / copy (as usual)
  wheel            scroll transcript (not the prompt)
  ↑↓ / PgUp PgDn   scroll transcript

Attach
  @path            attach file or folder
  @path#L10-40     attach line range
  @Symbol          jump to symbol

Commands"
        }
        Msg::SessionUnknownCmd => "unknown command; /help",
        Msg::SessionUsageRun => "usage: /run <prompt>",
        Msg::SessionNoGoals => "no plan goals",
        Msg::SessionBadKind => "bad kind",
        Msg::SessionEmptyResponse => "(empty response)",
        Msg::SessionError => "error - see transcript",
        Msg::SessionThreadExit => "agent thread exited",
        Msg::SupportedWord => "supported",
        Msg::UnsupportedWord => "unsupported",
        Msg::PermTitle => " allow tool? ",
        Msg::PermPrompt => "permission required - y/n",
        Msg::ComposerPlaceholder => "Ask tester...  (@file  /commands)",
        Msg::EmptyTranscript => "Start a prompt, or /help",
        Msg::EmptyWorktrees => "no git worktrees",
        Msg::DoctorTitle => "# tester doctor\n\n",
        Msg::DoctorOk => "ok",
        Msg::DoctorFail => "FAIL",
        Msg::ReportTitle => "# tester report\n\n",
        Msg::ReportProject => "project",
        Msg::ReportLanguage => "language",
        Msg::ReportPackage => "package",
        Msg::ReportWarnings => "## Warnings\n\n",
        Msg::ReportFailures => "## Failure summary\n\n",
        Msg::ReportResults => {
            "## Results\n\n| symbol | kind | file | attempts | status | first_pass | density | usefulness | command |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n"
        }
        Msg::ReportTrace => "Trace",
        Msg::ReportStatusPlanned => "planned",
        Msg::ReportStatusDryRun => "dry_run",
        Msg::ReportStatusWritten => "written",
        Msg::ReportStatusPassed => "passed",
        Msg::ReportStatusUnfixed => "unfixed",
        Msg::ReportStatusSkipped => "skipped",
        Msg::ReportStatusFailedPath => "failed_path",

        Msg::BadgeThinking => "Thinking",
        Msg::BadgeWorking => "Working",
        Msg::BadgePerm => "perm",
        Msg::BadgePlan => "PLAN",
        Msg::StatusIdle => "idle",
        Msg::CtxGauge => "ctx {used}/{limit}",
        Msg::PermWaitYn => "wait y/N",
        Msg::StatusHelpHint => " /help",
        Msg::StatusOpenEditor => "Ctrl+E - open $EDITOR",
        Msg::TuiDryRunBadge => "dry-run",
        Msg::TuiNoProject => "no project",
        Msg::WinTitleBusy => "testerbusy{model}",
        Msg::WinTitleIdle => "testeridle",
        Msg::WinTitlePicker => "testeropen project",
        Msg::PermAllow => "allow",
        Msg::PermDeny => "deny",
        Msg::ComposerRevSearch => "bck-i-search: ",

        Msg::SessionOpenFailed => "session open failed: {err}",
        Msg::AuditFailed => "audit failed: {err}",
        Msg::ScanFailed => "scan failed: {err}",
        Msg::ScanSummary => "language={lang} package={pkg} cmd={cmd}",
        Msg::PlanFailed => "plan failed: {err}",
        Msg::GitCheckoutOk => "checked out {branch}",
        Msg::GitCheckoutFailed => "checkout failed: {err}",
        Msg::GitCheckoutError => "checkout error: {err}",
        Msg::ProposePlanApproved => "/propose_plan (approved)",
        Msg::ProposePlanTitle => "plan ({kind})",
        Msg::ProposePlanPrompt => "Execute this test plan:\n{body}\nReply with progress.",
        Msg::UsageGrep => "usage: /grep <pattern>",
        Msg::UsageSymbol => "usage: /symbol <name>",
        Msg::UsageRefs => "usage: /refs <name>",
        Msg::UsageSearch => "usage: /search <natural query>",
        Msg::UsageShowFile => "usage: /show_file <path>",
        Msg::GrepFailed => "grep failed: {err}",
        Msg::ToolMissing => "{name} tool missing",
        Msg::ToolFailed => "{name} failed: {err}",
        Msg::IndexRebuilt => "index rebuilt: {n} symbols",
        Msg::IndexRebuildFailed => "index rebuild failed: {err}",
        Msg::IndexFailed => "index failed: {err}",
        Msg::SessionCmdError => "/session: {err}",
        Msg::PruneUsage => "usage: /prune <id|-w|-o <days>|all>",
        Msg::PruneDone => "pruned {n} session(s)",
        Msg::PruneError => "/prune: {err}",
        Msg::SessionSwitchFailed => "session switch failed: {err}",
        Msg::SessionOutcomeOk => "ok",
        Msg::SessionOutcomeCancelled => "cancelled",
        Msg::SessionOutcomeLine => "turns={turns} tools={tools}{status}",
        Msg::EditorFailed => "editor: {err}",

        Msg::ManualContent => {
            "\
# Tester manual

## Navigation
- Enter - send prompt
- @path / @Symbol - attach
- /commands - slash palette
- Esc Esc - cancel / close overlay
- Ctrl+C Ctrl+C - quit
- Ctrl+R - reverse history search
- Ctrl+X Ctrl+E - external editor

## Slash
/scan /plan /run /grep /symbol /refs /search /index /map /show_file /list_files
/manual /diff /branch /recent /session /prune /propose_plan /audit /clear /exit

## Analysis tools
find_symbol, project_map, graph_callers, semantic_search (opt-in embeddings)

## Approvals
y/N for write/shellplan approve/reject via /propose_plan
"
        }
        Msg::ManualTocTitle => " TOC ",
        Msg::ManualTitleSearching => " /manual  Alt+S search: {q}_ ",
        Msg::ManualTitleIdle => " /manual  Alt+S searchEsc close ",
        Msg::ManualSearchNoMatches => "(no matches)",
        Msg::PlanOverlayFooter => " a approver rejecte editEsc cancel ",
        Msg::PlanOverlayTitle => " propose_plan ",
        Msg::DiffUnavailable => "(git diff unavailable)",
        Msg::DiffClean => "(clean working tree)",
        Msg::DiffTitle => "diff",
        Msg::DiffOverlayChrome => " {title}Esc closePgUp/Dn ",
        Msg::BranchNone => "(no git branches)",
        Msg::BranchItemTag => "branch",
        Msg::BranchOverlayTitle => " branch switcherEnter checkoutEsc ",
        Msg::SessionsNone => "(no sessions)",
        Msg::SessionsNewHint => "n = new",
        Msg::SessionTitleEmpty => "(empty)",
        Msg::SessionsOverlayTitle => " sessionsEnter openn newEsc close ",
        Msg::RecentHasTester => "has .tester",
        Msg::RecentProject => "project",
        Msg::RecentOverlayTitle => " recent projectsEnter openEsc close ",

        Msg::PickerOkStatus => "ok: {path} ({word})",
        Msg::PickerWarnStatus => "warn: {err}",
        Msg::ReplBanner => "tester chat - agent REPL. /help for commands, Ctrl+C to quit.",
        Msg::ReplPrompt => "tester> ",
        Msg::ReplSlashHelp => {
            "\
Commands:
  /scan
  /plan [kind]
  /run <prompt>
  /clear
  /audit
  /grep <pat>
  /symbol <name>
  /refs <name>
  /search <q>
  /index status|rebuild
  /map
  /list_files
  /show_file <path>
  /exit"
        }
        Msg::AuditEmpty => "audit: empty",
        Msg::AuditHeader => "audit: {n} actions",
    }
}
