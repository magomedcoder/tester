//! Runtime localization (English / Russian).

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

pub fn current() -> Locale {
    match LOCALE.load(Ordering::Relaxed) {
        x if x == Locale::Ru as u8 => Locale::Ru,
        _ => Locale::En,
    }
}

pub fn set(locale: Locale) {
    LOCALE.store(locale as u8, Ordering::Relaxed);
}

pub fn is_forced() -> bool {
    FORCED.load(Ordering::Relaxed)
}

/// Priority: CLI `--locale` -> `TESTER_LOCALE` -> `LANG`/`LC_ALL` -> English.
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
    // CLI
    CliAbout,
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
    NextHintRunUnit,

    // TUI common
    TuiNeedsTty,
    FooterHome,
    FooterPicker,
    FooterSession,
    TitleHome,
    TitleFolders,
    TitleStatus,
    TitleTranscript,
    TitleComposer,
    TitleCommands,
    TitleSession,
    TitleOpenProject,
    NoProject,
    ProjectLabel,
    HomeHint,
    HomeStatusIdle,
    HomeStatusSelected,
    HomeStatusWarn,
    DiscoverFailed,
    PickerStatusProject,
    PickerStatusNav,
    PickerFilter,
    PickerProjectTag,
    SessionFailedOpen,
    SessionFixHint,
    SessionStatusDryRun,
    SessionStatusAuto,
    SessionStatusBlocked,
    SessionCleared,
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
    RoleYou,
    RoleAssistant,
    RoleTool,
    RoleSystem,
    SupportedWord,
    UnsupportedWord,
}

pub fn t(msg: Msg) -> &'static str {
    match current() {
        Locale::En => en::t(msg),
        Locale::Ru => ru::t(msg),
    }
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
