mod en;
mod ru;

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Locale {
    #[default]
    En = 0,
    Ru = 1,
}

impl Locale {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" | "english" => Some(Self::En),
            "ru" | "ru-ru" | "ru_ru" | "russian" | "рус" => Some(Self::Ru),
            other if other.starts_with("en") => Some(Self::En),
            other if other.starts_with("ru") => Some(Self::Ru),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ru => "ru",
        }
    }
}

static LOCALE: AtomicU8 = AtomicU8::new(Locale::En as u8);
static FORCED: AtomicBool = AtomicBool::new(false);

pub(crate) fn current() -> Locale {
    match LOCALE.load(Ordering::Relaxed) {
        x if x == Locale::Ru as u8 => Locale::Ru,
        _ => Locale::En,
    }
}

pub(crate) fn set(locale: Locale) {
    LOCALE.store(locale as u8, Ordering::Relaxed);
}

fn is_forced() -> bool {
    FORCED.load(Ordering::Relaxed)
}

pub fn init_from_env(cli_locale: Option<&str>) {
    if let Some(raw) = cli_locale {
        if let Some(loc) = Locale::parse(raw) {
            FORCED.store(true, Ordering::Relaxed);
            set(loc);
            return;
        }
    }
    if let Ok(raw) = std::env::var("TESTER_LOCALE") {
        if let Some(loc) = Locale::parse(&raw) {
            FORCED.store(true, Ordering::Relaxed);
            set(loc);
            return;
        }
    }
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(raw) = std::env::var(key) {
            if let Some(loc) = Locale::parse(&raw) {
                set(loc);
                return;
            }
        }
    }
    set(Locale::En);
}

pub fn apply_config_locale(raw: Option<&str>) {
    if is_forced() {
        return;
    }
    if let Some(raw) = raw {
        if let Some(loc) = Locale::parse(raw) {
            set(loc);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Msg {
    ErrorPrefix,
    CancelHint,
    NoGoals,
    WarningPrefix,
    Language,
    Package,
    Supported,
    TestCommand,
    TestStyle,
    HasTests,
    UnitFramework,
    UiFramework,
    UiReady,
    Capabilities,
    Note,
    PlanArrow,
    LlmUnavailable,
    UnsupportedLanguage,
    PlaywrightInit,
    ConfigLabel,
    TestsLabel,

    TuiNeedsTty,
    TitleCommands,
    TitleOpenProject,
    TitleFiles,
    TitleWorktrees,
    PopupNoMatches,
    AttachHint,
    SlashHelp,
    SlashScan,
    SlashPlan,
    SlashRun,
    SlashClear,
    SlashAudit,
    SlashExit,
    SlashGrep,
    SlashSymbol,
    SlashRefs,
    SlashSearch,
    SlashIndex,
    SlashMap,
    SlashListFiles,
    SlashShowFile,
    SlashManual,
    SlashRecent,
    SlashSession,
    SlashPrune,
    SlashDiff,
    SlashBranch,
    SlashProposePlan,
    SlashSecurity,
    SecurityOverlayTitle,
    PickerStatusProject,
    PickerStatusNav,
    PickerStatusWorktrees,
    PickerFilter,
    PickerProjectTag,
    PickerWorktreeTag,
    SessionFailedOpen,
    SessionFixHint,
    SessionStatusDryRun,
    SessionStatusAuto,
    SessionStatusBlocked,
    SessionCleared,
    SessionSwitched,
    SessionRunning,
    SessionCancelReq,
    SessionHelp,
    SessionUnknownCmd,
    SessionUsageRun,
    SessionNoGoals,
    SessionBadKind,
    SessionEmptyResponse,
    SessionError,
    SessionThreadExit,
    SupportedWord,
    UnsupportedWord,
    PermTitle,
    PermPrompt,
    ComposerPlaceholder,
    EmptyTranscript,
    EmptyWorktrees,
    DoctorTitle,
    DoctorOk,
    DoctorFail,
    ReportTitle,
    ReportProject,
    ReportLanguage,
    ReportPackage,
    ReportWarnings,
    ReportFailures,
    ReportResults,
    ReportTrace,
    ReportStatusPlanned,
    ReportStatusDryRun,
    ReportStatusWritten,
    ReportStatusPassed,
    ReportStatusUnfixed,
    ReportStatusSkipped,
    ReportStatusFailedPath,

    BadgeThinking,
    BadgeWorking,
    BadgePerm,
    BadgePlan,
    StatusIdle,
    CtxGauge,
    PermWaitYn,
    StatusHelpHint,
    StatusOpenEditor,
    TuiDryRunBadge,
    TuiNoProject,
    WinTitleBusy,
    WinTitleIdle,
    WinTitlePicker,
    PermAllow,
    PermDeny,
    ComposerRevSearch,

    SessionOpenFailed,
    AuditFailed,
    ScanFailed,
    ScanSummary,
    PlanFailed,
    GitCheckoutOk,
    GitCheckoutFailed,
    GitCheckoutError,
    ProposePlanApproved,
    ProposePlanTitle,
    ProposePlanPrompt,
    UsageGrep,
    UsageSymbol,
    UsageRefs,
    UsageSearch,
    UsageShowFile,
    GrepFailed,
    ToolMissing,
    ToolFailed,
    IndexRebuilt,
    IndexRebuildFailed,
    IndexFailed,
    SessionCmdError,
    PruneUsage,
    PruneDone,
    PruneError,
    SessionSwitchFailed,
    SessionOutcomeOk,
    SessionOutcomeCancelled,
    SessionOutcomeLine,
    EditorFailed,

    ManualContent,
    ManualTocTitle,
    ManualTitleSearching,
    ManualTitleIdle,
    ManualSearchNoMatches,
    PlanOverlayFooter,
    PlanOverlayTitle,
    DiffUnavailable,
    DiffClean,
    DiffTitle,
    DiffOverlayChrome,
    BranchNone,
    BranchItemTag,
    BranchOverlayTitle,
    SessionsNone,
    SessionsNewHint,
    SessionTitleEmpty,
    SessionsOverlayTitle,
    RecentHasTester,
    RecentProject,
    RecentOverlayTitle,

    PickerOkStatus,
    PickerWarnStatus,
    ReplBanner,
    ReplPrompt,
    ReplSlashHelp,
    AuditEmpty,
    AuditHeader,

    PlanProgress,
    PlanItemNotFound,
    PlainNeedsPath,
    PruneNeedsArg,
    PruneCli,
    SecurityPatchNote,
    SecurityApplied,
    SecurityDryRunHint,
    FindingNotFound,
    UsageLine,
    ReplTurnsLine,
    ItemWritten,
    ItemPassed,
    PlanShowEmpty,
    LlmClientCreate,
    ErrPathEscapes,
    ErrPathForbidden,
    ErrPathNeedsUpdate,
    ErrManifestWrite,
    ErrGateNoAssertion,
    ErrGateWeakTest,
    ErrGateForbiddenNetwork,
    ErrGateUnitNetwork,
    DoctorNoAdapter,
    DoctorNotRequired,
    DoctorGradlewFound,
    DoctorGradlewMissing,
    DoctorHttp,
    DoctorUiState,
    DoctorSecurityRules,
    SecReportTitle,
    SecCounts,
    SecNotes,
    SecFindings,
    SecNoFindings,
    SecScanSummary,
    SecScanFailed,
    SecFixLine,
    SecNoteLine,
    PermPromptCli,
    IoRead,
    IoWrite,
    IoCreate,
    IoParse,
    AgentDeadline,
    MaxToolCalls,
    UnknownTool,
    AgentTurnLimit,
    AgentStepModel,
    AgentStepDone,
    AgentStepTool,
    SecMetaLine,
    SecFindingMeta,
    WarnFlutterIntegration,
    WarnUiSkipped,
    WarnPlaywrightMissing,
    NoteDockerAllowed,
    NoteApiSurface,
    NoteCppKinds,
    NoteAndroidOptIn,
    NoteVenv,
    NoteLaravel,

    ErrContextTooLarge,
    ErrParseDraftFromLlm,
    ErrEmptyValidateCommand,
    ErrUiBaseUnavailable,
    ErrUiStartEarly,
    ErrWaitUrlTimeout,
    ErrValidateTimeout,
    ErrBugInputRequired,
    ErrNoRepairState,
    ErrNoMutationTarget,
    ErrNoMutationApplied,
    ErrNoFixturesDir,
    ErrNoAdapterForLang,
    ErrLangUndetected,
    ErrProjectManifestNotFound,
    ErrPatchFragmentNotFound,
    ErrToolArgRequired,
    ErrPlanSymbolNotFound,
    ErrNoPlanItemForSymbol,
    ErrNoLastRepairJson,
    ErrEmptyAdapterArgv,
    ErrDockerDisabled,
    ErrDockerUnavailable,
    ErrLlmAccessDenied,
    ErrLlmHttp,
    ErrLlmHttpStatus,
    ErrLlmEmptyResponse,
    ErrEmbeddingsHttp,
    ErrEmbeddingsNoData,
    ErrEmbeddingsNoEmbedding,
    ErrEmbeddingsDisabled,
    ErrEmbeddingsModelRequired,
    ErrSemanticSearchNeedsEmbeddings,
    ErrFileNotFound,
    OutlineNoSymbols,
    SessionIdEmpty,
    SessionIdNoMatch,
    SessionIdAmbiguous,
}

pub fn t(msg: Msg) -> &'static str {
    match current() {
        Locale::En => en::t(msg),
        Locale::Ru => ru::t(msg),
    }
}

pub fn io_ctx(msg: Msg, path: &std::path::Path) -> String {
    tr(msg, &[("path", &path.display().to_string())])
}

pub fn tr(msg: Msg, pairs: &[(&str, &str)]) -> String {
    let mut s = t(msg).to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_locale() {
        assert_eq!(Locale::parse("en"), Some(Locale::En));
        assert_eq!(Locale::parse("ru_RU.UTF-8"), Some(Locale::Ru));
        assert_eq!(Locale::parse("EN-us"), Some(Locale::En));
        assert!(Locale::parse("de").is_none());
    }

    #[test]
    fn catalogs_differ() {
        let prev = current();
        set(Locale::En);
        let en_s = t(Msg::NoGoals);
        set(Locale::Ru);
        let ru_s = t(Msg::NoGoals);
        set(prev);
        assert_ne!(en_s, ru_s);
        assert!(ru_s.contains("цел") || ru_s.contains("нет"));
    }
}
