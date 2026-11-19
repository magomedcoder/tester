use crate::model::{ItemStatus, RunReport};
use anyhow::Error;
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

pub fn exit_from_error(err: &Error) -> ExitCode {
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
