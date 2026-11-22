use crate::i18n::{self, Msg};
use crate::model::{ItemStatus, RunReport};
use anyhow::Error;
use std::fmt;
use std::process::ExitCode as StdExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(pub u8);

impl From<ExitCode> for StdExitCode {
    fn from(value: ExitCode) -> Self {
        StdExitCode::from(value.0)
    }
}

impl ExitCode {
    pub const OK: Self = Self(0);
    pub const ERROR: Self = Self(1);
    pub const NO_GOALS: Self = Self(2);
    pub const LLM_UNAVAILABLE: Self = Self(3);
    pub const UNFIXED: Self = Self(4);
    pub const PATH_POLICY: Self = Self(5);
}

#[derive(Debug)]
pub enum AppError {
    NoGoals,
    LlmUnavailable,
    PathEscapesRoot { path: String },
    PathForbidden { detail: String },
    PathNeedsUpdate,
    ManifestWriteRefused { name: String },
    GateNoAssertion,
    GateWeakTest,
    GateForbiddenNetwork,
    GateUnitNetwork,
    PlainNeedsPath,
    PruneNeedsArg,
    PlanItemNotFound { symbol: String },
    FindingNotFound { id: String },
    UnsupportedLanguage { lang: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoGoals => f.write_str(i18n::t(Msg::NoGoals)),
            Self::LlmUnavailable => f.write_str(i18n::t(Msg::LlmUnavailable)),
            Self::PathEscapesRoot { path } => {
                f.write_str(&i18n::tr(Msg::ErrPathEscapes, &[("path", path)]))
            }
            Self::PathForbidden { detail } => {
                f.write_str(&i18n::tr(Msg::ErrPathForbidden, &[("detail", detail)]))
            }
            Self::PathNeedsUpdate => f.write_str(i18n::t(Msg::ErrPathNeedsUpdate)),
            Self::ManifestWriteRefused { name } => {
                f.write_str(&i18n::tr(Msg::ErrManifestWrite, &[("name", name)]))
            }
            Self::GateNoAssertion => f.write_str(i18n::t(Msg::ErrGateNoAssertion)),
            Self::GateWeakTest => f.write_str(i18n::t(Msg::ErrGateWeakTest)),
            Self::GateForbiddenNetwork => f.write_str(i18n::t(Msg::ErrGateForbiddenNetwork)),
            Self::GateUnitNetwork => f.write_str(i18n::t(Msg::ErrGateUnitNetwork)),
            Self::PlainNeedsPath => f.write_str(i18n::t(Msg::PlainNeedsPath)),
            Self::PruneNeedsArg => f.write_str(i18n::t(Msg::PruneNeedsArg)),
            Self::PlanItemNotFound { symbol } => {
                f.write_str(&i18n::tr(Msg::PlanItemNotFound, &[("symbol", symbol)]))
            }
            Self::FindingNotFound { id } => {
                f.write_str(&i18n::tr(Msg::FindingNotFound, &[("id", id)]))
            }
            Self::UnsupportedLanguage { lang } => {
                write!(f, "{}: {lang}", i18n::t(Msg::UnsupportedLanguage))
            }
        }
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NoGoals => ExitCode::NO_GOALS,
            Self::LlmUnavailable => ExitCode::LLM_UNAVAILABLE,
            Self::PathEscapesRoot { .. }
            | Self::PathForbidden { .. }
            | Self::PathNeedsUpdate
            | Self::ManifestWriteRefused { .. }
            | Self::GateForbiddenNetwork => ExitCode::PATH_POLICY,
            Self::GateNoAssertion
            | Self::GateWeakTest
            | Self::GateUnitNetwork
            | Self::PlainNeedsPath
            | Self::PruneNeedsArg
            | Self::PlanItemNotFound { .. }
            | Self::FindingNotFound { .. }
            | Self::UnsupportedLanguage { .. } => ExitCode::ERROR,
        }
    }
}

pub fn exit_from_error(err: &Error) -> ExitCode {
    if let Some(app) = err.downcast_ref::<AppError>() {
        return app.exit_code();
    }
    let msg = format!("{err:#}").to_ascii_lowercase();
    if msg.contains("llm")
        || msg.contains("chat/completions")
        || msg.contains("401")
        || msg.contains("403")
        || msg.contains("connect")
        || msg.contains("timed out")
        || msg.contains("timeout")
    {
        return ExitCode::LLM_UNAVAILABLE;
    }
    if msg.contains("отказ")
        || msg.contains("запрещён")
        || msg.contains("политик")
        || msg.contains("--update")
        || msg.contains("выходит за корень")
        || msg.contains("refused")
        || msg.contains("forbidden")
        || msg.contains("escapes")
        || msg.contains("root")
        || msg.contains("policy")
    {
        return ExitCode::PATH_POLICY;
    }
    if msg.contains("нет целей") || msg.contains("no goals") {
        return ExitCode::NO_GOALS;
    }
    ExitCode::ERROR
}

pub fn exit_from_report(report: &RunReport) -> ExitCode {
    if report.items.is_empty() {
        return ExitCode::NO_GOALS;
    }
    if report
        .items
        .iter()
        .any(|i| i.status == ItemStatus::FailedPath)
    {
        return ExitCode::PATH_POLICY;
    }
    if report.items.iter().any(|i| i.status == ItemStatus::Unfixed) {
        return ExitCode::UNFIXED;
    }
    ExitCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ReportItem, TestKind};
    use anyhow::anyhow;
    use std::path::PathBuf;

    #[test]
    fn maps_llm_errors() {
        let err = anyhow!("llm HTTP 401: forbidden");
        assert_eq!(exit_from_error(&err), ExitCode::LLM_UNAVAILABLE);
    }

    #[test]
    fn maps_path_policy() {
        let err = anyhow!("отказ перезаписать существующий файл без --update");
        assert_eq!(exit_from_error(&err), ExitCode::PATH_POLICY);
    }

    #[test]
    fn maps_path_policy_english_fallback() {
        let err = anyhow!("refusing: path escapes project root");
        assert_eq!(exit_from_error(&err), ExitCode::PATH_POLICY);
    }

    #[test]
    fn typed_errors_map_to_exit_codes() {
        let cases = [
            (AppError::NoGoals, ExitCode::NO_GOALS),
            (AppError::LlmUnavailable, ExitCode::LLM_UNAVAILABLE),
            (AppError::PathNeedsUpdate, ExitCode::PATH_POLICY),
            (
                AppError::PathEscapesRoot { path: "x".into() },
                ExitCode::PATH_POLICY,
            ),
            (
                AppError::ManifestWriteRefused {
                    name: "Cargo.toml".into(),
                },
                ExitCode::PATH_POLICY,
            ),
            (AppError::GateNoAssertion, ExitCode::ERROR),
            (AppError::PruneNeedsArg, ExitCode::ERROR),
        ];
        for (e, code) in cases {
            assert_eq!(e.exit_code(), code);
            assert_eq!(exit_from_error(&anyhow::Error::new(e)), code);
        }
    }

    #[test]
    fn typed_error_survives_context() {
        let err = anyhow::Error::new(AppError::PathNeedsUpdate).context("tests/t.rs");
        assert_eq!(exit_from_error(&err), ExitCode::PATH_POLICY);
    }

    #[test]
    fn display_is_localized_not_used_for_exit() {
        let e = AppError::FindingNotFound { id: "F-1".into() };
        assert!(e.to_string().contains("F-1"));
        assert_eq!(e.exit_code(), ExitCode::ERROR);
    }

    #[test]
    fn maps_report_unfixed() {
        let report = RunReport {
            items: vec![ReportItem {
                symbol: "x".into(),
                kind: TestKind::Unit,
                file: PathBuf::from("t.rs"),
                attempts: 3,
                status: ItemStatus::Unfixed,
                command: String::new(),
                message: String::new(),
                trace_path: None,
                first_pass: None,
                assert_density: None,
                usefulness: None,
            }],
            warnings: vec![],
            metrics: Default::default(),
        };
        assert_eq!(exit_from_report(&report), ExitCode::UNFIXED);
    }
}
