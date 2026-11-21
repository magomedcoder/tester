use super::Msg;

pub fn t(msg: Msg) -> &'static str {
    match msg {
        Msg::CliAbout => "LLM agent for generating and fixing tests",
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
        Msg::NextHintRunUnit => "next: tester run <path> --kind unit",

        Msg::TuiNeedsTty => {
            "tui requires an interactive terminal; use `tester chat <PATH>` or run in a TTY"
        }
        Msg::FooterHome => " Enter session    Ctrl+O project    q quit ",
        Msg::FooterPicker => {
            " ↑↓  Enter open    Ctrl+Enter/Ctrl+S select    -> worktrees    Esc back    type to filter "
        }
        Msg::FooterSession => {
            " Enter send    @file    /commands    y/N approve    Esc cancel/quit    Ctrl+O project "
        }
        Msg::TitleHome => " home ",
        Msg::TitleFolders => " folders ",
        Msg::TitleStatus => " status ",
        Msg::TitleTranscript => " transcript ",
        Msg::TitleComposer => " composer ",
        Msg::TitleCommands => " /commands ",
        Msg::TitleSession => " session ",
        Msg::TitleOpenProject => " open project ",
        Msg::TitleAttach => " attached ",
        Msg::TitleFiles => " @files ",
        Msg::TitleWorktrees => " worktrees ",
        Msg::NoProject => "(no project selected)",
        Msg::ProjectLabel => "Project",
        Msg::HomeHint => "Open a project to start the session.",
        Msg::HomeStatusIdle => "Ctrl+O open project  q quit",
        Msg::HomeStatusSelected => "project selected  Ctrl+O switch  q quit",
        Msg::HomeStatusWarn => "path set with warnings  Ctrl+O switch  q quit",
        Msg::DiscoverFailed => "discover failed",
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
        Msg::SessionStatusBlocked => {
            "write/shell need y/N confirm  Enter send  @file  /help"
        }
        Msg::SessionCleared => "session cleared",
        Msg::SessionRunning => "running... Esc to cancel",
        Msg::SessionCancelReq => "cancel requested...",
        Msg::SessionHelp => {
            "commands: /scan /plan [kind] /run <prompt> /config /clear /audit /exit  attach with @path or @path#L20-45"
        }
        Msg::SessionUnknownCmd => "unknown command; /help",
        Msg::SessionUsageRun => "usage: /run <prompt>",
        Msg::SessionNoGoals => "no plan goals",
        Msg::SessionBadKind => "bad kind",
        Msg::SessionEmptyResponse => "(empty response)",
        Msg::SessionError => "error - see transcript",
        Msg::SessionThreadExit => "agent thread exited",
        Msg::RoleYou => "you",
        Msg::RoleAssistant => "assistant",
        Msg::RoleTool => "tool",
        Msg::RoleSystem => "system",
        Msg::SupportedWord => "supported",
        Msg::UnsupportedWord => "unsupported",
        Msg::PermTitle => " allow tool? ",
        Msg::PermHint => "y / Enter = allow    n / Esc = deny",
        Msg::PermPrompt => "permission required - y/n",
        Msg::DoctorTitle => "# tester doctor\n\n",
        Msg::DoctorOk => "ok",
        Msg::DoctorFail => "FAIL",
        Msg::ReportTitle => "# tester report\n\n",
        Msg::ReportProject => "project",
        Msg::ReportLanguage => "language",
        Msg::ReportPackage => "package",
        Msg::ReportWarnings => "## Warnings\n\n",
        Msg::ReportFailures => "## Failure summary\n\n",
        Msg::ReportResults => "## Results\n\n| symbol | kind | file | attempts | status | first_pass | density | usefulness | command |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
        Msg::ReportTrace => "Trace",
        Msg::ReportStatusPlanned => "planned",
        Msg::ReportStatusDryRun => "dry_run",
        Msg::ReportStatusWritten => "written",
        Msg::ReportStatusPassed => "passed",
        Msg::ReportStatusUnfixed => "unfixed",
        Msg::ReportStatusSkipped => "skipped",
        Msg::ReportStatusFailedPath => "failed_path",
    }
}
