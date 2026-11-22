use crate::cli::args::OutputFormat;
use anyhow::{Context, Result, anyhow};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tester_agent::{AgentContext, open_context_with_policy};
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
use tester_core::model::{ProjectProfile, RunReport, TestKind};
use tester_lang::discover::discover;
use tester_llm::{Llm, create_client};
use tester_pipeline::plan::PlanOptions;
use tester_pipeline::report::{render_json, render_markdown};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub(crate) static CANCEL: AtomicBool = AtomicBool::new(false);

pub(crate) fn prepare_project(path: &PathBuf) -> Result<ProjectProfile> {
    let mut profile = discover(path)?;
    tester_lang::adapter::ensure_scaffold(&profile)?;
    if profile.language == tester_core::model::Language::Cpp {
        profile = discover(path)?;
    }
    Ok(profile)
}

pub(crate) fn plan_opts(
    base: Option<String>,
    bug: Option<String>,
    bug_file: Option<PathBuf>,
    append: bool,
    max_items: Option<usize>,
    coverage: Option<PathBuf>,
) -> Result<PlanOptions> {
    let bug_text = match (bug, bug_file) {
        (Some(b), _) => Some(b),
        (None, Some(path)) => Some(std::fs::read_to_string(path)?),
        _ => None,
    };
    let frames = bug_text
        .as_ref()
        .map(|t| tester_core::stacktrace::parse_stacktrace(t))
        .unwrap_or_default();
    Ok(PlanOptions {
        diff_base: base,
        bug_text,
        prefer_append: append,
        max_items,
        coverage_path: coverage,
        stack_frames: frames,
        reproduce_only: false,
    })
}

pub(crate) fn parse_kind(kind: &str) -> Result<TestKind> {
    kind.parse().map_err(|e: String| anyhow!(e))
}

pub(crate) fn load_config(profile: &ProjectProfile) -> Result<Config> {
    Config::load(&profile.root)
}

pub(crate) fn need_llm(config: &Config) -> Result<Arc<dyn Llm>> {
    create_client(&config.llm).context("создание клиента llm")
}

pub(crate) fn apply_config_flags(profile: &mut ProjectProfile, config: &Config) {
    if config.run.android_opt_in {
        profile.android_opt_in = true;
    }
    if config.run.allow_docker {
        profile.notes.push(
            "allow_docker=true: tool docker_info доступен агенту; внешние сервисы по-прежнему не поднимаются автоматически"
                .into(),
        );
    }
}

pub(crate) fn build_agent_ctx(path: &PathBuf, dry_run: bool, stream: bool) -> Result<AgentContext> {
    open_context_with_policy(path, dry_run, stream, false)
}

pub(crate) fn mirror_cancel(flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            if CANCEL.load(Ordering::SeqCst) {
                flag.store(true, Ordering::SeqCst);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

pub(crate) fn emit_report(
    profile: &ProjectProfile,
    report: &RunReport,
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = render_json(profile, report);
            println!("{json}");
            let md = render_markdown(profile, report);
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "{md}");
        }
        OutputFormat::Text => {
            print!("{}", render_markdown(profile, report));
        }
    }
    Ok(())
}

pub(crate) fn install_ctrlc() {
    let _ = ctrlc::set_handler(|| {
        CANCEL.store(true, Ordering::SeqCst);
        eprintln!("{}", i18n::t(Msg::CancelHint));
    });
}

pub(crate) fn init_tracing(debug: bool, for_tui: bool) {
    let default = if debug { "debug" } else { "info" };
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(std::env::var("RUST_LOG").unwrap_or_else(|_| default.into()));
    if for_tui {
        if let Some(path) = tui_log_path() {
            if let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                let _ = tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .try_init();
                return;
            }
        }
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

pub(crate) fn tui_log_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TESTER_TUI_LOG") {
        let path = PathBuf::from(p);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        return Some(path);
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let dir = PathBuf::from(home).join(".tester").join("logs");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("tui.log"))
}
