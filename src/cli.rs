use crate::agent::{AgentContext, AuditLog, PermissionPolicy, run_one_shot, run_repl};
use crate::i18n::{self, Msg};
use crate::apply::apply_draft;
use crate::config::Config;
use crate::discover::discover;
use crate::error::{ExitCode, exit_from_error};
use crate::eval::{eval_report, eval_suite, render_text as render_eval};
use crate::generate::generate_draft;
use crate::llm::{Llm, create_client};
use crate::model::{ItemStatus, RunReport, TestKind};
use crate::plan::{PlanOptions, build_plan, build_plan_with};
use crate::repair::{LastRepairState, load_state, run_plan, save_state};
use crate::report::{exit_for, render_json, render_markdown, render_usage};
use crate::validate::write_bytes;
use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode as StdExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "tester",
    version,
    about = "LLM agent for generating and fixing tests"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[arg(long, global = true)]
    pub debug: bool,

    #[arg(long, global = true, value_name = "LOCALE")]
    pub locale: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scan {
        path: PathBuf,
    },

    Plan {
        path: PathBuf,
        #[arg(long, default_value = "all")]
        kind: String,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        bug: Option<String>,
        #[arg(long)]
        bug_file: Option<PathBuf>,
        #[arg(long)]
        append: bool,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        max_items: Option<usize>,
        #[arg(long)]
        coverage: Option<PathBuf>,
        #[arg(long)]
        show: bool,
        #[arg(long)]
        apply: bool,
    },

    Generate {
        path: PathBuf,
        #[arg(long, default_value = "all")]
        kind: String,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        update: bool,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        bug: Option<String>,
        #[arg(long)]
        bug_file: Option<PathBuf>,
        #[arg(long)]
        append: bool,
        #[arg(long)]
        package: Option<String>,
    },

    Run {
        path: PathBuf,
        #[arg(long, default_value = "all")]
        kind: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        update: bool,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        bug: Option<String>,
        #[arg(long)]
        bug_file: Option<PathBuf>,
        #[arg(long)]
        append: bool,
        #[arg(long)]
        package: Option<String>,
    },

    Repair {
        path: PathBuf,
    },

    Eval {
        path: PathBuf,
        #[arg(long, default_value_t = 3)]
        max_symbols: usize,
        #[arg(long)]
        suite: bool,
        #[arg(long)]
        json: bool,
    },

    Agent {
        path: PathBuf,

        prompt: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        resume: bool,
        #[arg(long, default_value_t = true)]
        stream: bool,
        #[arg(long)]
        no_stream: bool,
    },

    Chat {
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = true)]
        stream: bool,
        #[arg(long)]
        no_stream: bool,
    },

    Tui {
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        resume: bool,
        #[arg(long)]
        plain: bool,
    },

    Debug {
        path: PathBuf,
        #[arg(long)]
        bug: Option<String>,
        #[arg(long)]
        bug_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
        #[arg(long)]
        reproduce_only: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        update: bool,
        #[arg(long, default_value = "unit")]
        kind: String,
        #[arg(long, default_value_t = 5)]
        max_items: usize,
    },

    Doctor {
        path: PathBuf,
    },

    #[command(subcommand)]
    Ui(UiCommands),
}

#[derive(Debug, Subcommand)]
pub enum UiCommands {
    Init { path: PathBuf },
    Doctor { path: PathBuf },
}

pub fn run() -> StdExitCode {
    let cli = Cli::parse();
    crate::i18n::init_from_env(cli.locale.as_deref());
    init_tracing(cli.debug);
    install_ctrlc();
    match run_cli(cli) {
        Ok(code) => code.into(),
        Err(err) => {
            eprintln!("{}{err:#}", i18n::t(Msg::ErrorPrefix));
            exit_from_error(&err).into()
        }
    }
}

static CANCEL: AtomicBool = AtomicBool::new(false);

fn install_ctrlc() {
    let _ = ctrlc::set_handler(|| {
        CANCEL.store(true, Ordering::SeqCst);
        eprintln!("{}", i18n::t(Msg::CancelHint));
    });
}

fn init_tracing(debug: bool) {
    let default = if debug { "debug" } else { "info" };
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(std::env::var("RUST_LOG").unwrap_or_else(|_| default.into()));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

fn run_cli(cli: Cli) -> Result<ExitCode> {
    let format = cli.format;
    match cli.command {
        Commands::Scan { path } => cmd_scan(&path, format),
        Commands::Plan {
            path,
            kind,
            base,
            bug,
            bug_file,
            append,
            package,
            max_items,
            coverage,
            show,
            apply,
        } => {
            if show {
                cmd_plan_show(&path, format)
            } else if apply {
                cmd_plan_apply(&path, false, false, format)
            } else {
                cmd_plan(
                    &path,
                    &kind,
                    format,
                    plan_opts(base, bug, bug_file, append, max_items, coverage)?,
                    package,
                )
            }
        }
        Commands::Generate {
            path,
            kind,
            write,
            update,
            base,
            bug,
            bug_file,
            append,
            package,
        } => cmd_generate(
            &path,
            &kind,
            write,
            update,
            format,
            plan_opts(base, bug, bug_file, append, None, None)?,
            package
        ),
        Commands::Run {
            path,
            kind,
            dry_run,
            update,
            base,
            bug,
            bug_file,
            append,
            package,
        } => cmd_run(
            &path,
            &kind,
            !dry_run,
            update,
            format,
            plan_opts(base, bug, bug_file, append, None, None)?,
            package
        ),
        Commands::Repair { path } => cmd_repair(&path, format),
        Commands::Eval {
            path,
            max_symbols,
            suite,
            json,
        } => cmd_eval(&path, max_symbols, suite, json, format),
        Commands::Agent {
            path,
            prompt,
            dry_run,
            resume,
            stream,
            no_stream,
        } => cmd_agent(
            &path,
            &prompt,
            dry_run,
            resume,
            stream && !no_stream
        ),
        Commands::Chat {
            path,
            dry_run,
            stream,
            no_stream,
        } => cmd_chat(&path, dry_run, stream && !no_stream),
        Commands::Tui {
            path,
            dry_run,
            resume,
            plain,
        } => cmd_tui(path, dry_run, resume, plain),
        Commands::Debug {
            path,
            bug,
            bug_file,
            stdin,
            reproduce_only,
            dry_run,
            update,
            kind,
            max_items,
        } => cmd_debug(
            &path,
            bug,
            bug_file.as_deref(),
            stdin,
            reproduce_only,
            dry_run,
            update,
            &kind,
            max_items,
            format
        ),
        Commands::Doctor { path } => cmd_doctor(&path, format),
        Commands::Ui(UiCommands::Init { path }) => cmd_ui_init(&path, format),
        Commands::Ui(UiCommands::Doctor { path }) => cmd_ui_doctor(&path, format),
    }
}

fn prepare_project(path: &PathBuf) -> Result<crate::model::ProjectProfile> {
    let mut profile = discover(path)?;
    crate::adapter::ensure_scaffold(&profile)?;
    if profile.language == crate::model::Language::Cpp {
        profile = discover(path)?;
    }
    Ok(profile)
}

fn plan_opts(
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
        .map(|t| crate::debug::parse_stacktrace(t))
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

fn parse_kind(kind: &str) -> Result<TestKind> {
    kind.parse().map_err(|e: String| anyhow!(e))
}

fn load_config(profile: &crate::model::ProjectProfile) -> Result<Config> {
    Config::load(&profile.root)
}

fn cmd_scan(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = discover(path)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&profile)?),
        OutputFormat::Text => {
            println!("{}: {}", i18n::t(Msg::Language), profile.language.as_str());
            println!("{}: {}", i18n::t(Msg::Package), profile.package_name);
            println!("{}: {}", i18n::t(Msg::Supported), profile.supported);
            println!("{}: {}", i18n::t(Msg::TestCommand), profile.test_command);
            println!("{}: {:?}", i18n::t(Msg::TestStyle), profile.test_style);
            println!("{}: {}", i18n::t(Msg::HasTests), profile.has_existing_tests);
            if let Some(fw) = &profile.unit_framework {
                println!("{}: {fw}", i18n::t(Msg::UnitFramework));
            }
            if let Some(fw) = &profile.ui_framework {
                println!("{}: {fw}", i18n::t(Msg::UiFramework));
            }
            println!("{}: {}", i18n::t(Msg::UiReady), profile.ui_ready);
            println!(
                "{}: {}",
                i18n::t(Msg::Capabilities),
                profile.capabilities.list().join(", ")
            );
            for n in &profile.notes {
                println!("{}: {n}", i18n::t(Msg::Note));
            }
        }
    }
    Ok(ExitCode::OK)
}

fn cmd_plan(
    path: &PathBuf,
    kind: &str,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>,
) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    let config = Config::load(&profile.root)?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        OutputFormat::Text => {
            if plan.items.is_empty() {
                println!("{}", i18n::t(Msg::NoGoals));
            }
            for item in &plan.items {
                println!(
                    "- [{}] {} {} {} ({})",
                    item.kind.as_str(),
                    item.gap.symbol.name,
                    i18n::t(Msg::PlanArrow),
                    item.target_path.display(),
                    item.gap.reason
                );
            }
            for w in &plan.warnings {
                eprintln!("{}{w}", i18n::t(Msg::WarningPrefix));
            }
        }
    }
    Ok(if plan.items.is_empty() {
        ExitCode::NO_GOALS
    } else {
        ExitCode::OK
    })
}

fn need_llm(config: &Config) -> Result<Arc<dyn Llm>> {
    create_client(&config.llm).context("создание клиента llm")
}

fn apply_config_flags(profile: &mut crate::model::ProjectProfile, config: &Config) {
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

fn build_agent_ctx(
    path: &PathBuf,
    dry_run: bool,
    stream: bool) -> Result<AgentContext> {
    let mut profile = prepare_project(path)?;
    let config = load_config(&profile)?;
    apply_config_flags(&mut profile, &config);
    let llm = need_llm(&config)?;
    let yes = config.run.agent_auto_approve;
    Ok(AgentContext {
        profile,
        config,
        llm,
        dry_run,
        yes,
        stream,
        cancel: Arc::new(AtomicBool::new(false)),
        audit: AuditLog::new(),
        permissions: PermissionPolicy::from_flags(yes, dry_run),
    })
}

fn mirror_cancel(flag: Arc<AtomicBool>) {
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

fn cmd_agent(
    path: &PathBuf,
    prompt: &str,
    dry_run: bool,
    resume: bool,
    stream: bool) -> Result<ExitCode> {
    let ctx = build_agent_ctx(path, dry_run, stream)?;
    mirror_cancel(Arc::clone(&ctx.cancel));

    let outcome = run_one_shot(&ctx, prompt, resume)?;
    println!("{}", outcome.final_text);
    eprintln!("- {}", render_usage(&outcome.usage));
    eprintln!(
        "- turns={} tools={} {}",
        outcome.turns,
        outcome.tool_calls,
        if outcome.cancelled { "cancelled" } else { "ok" }
    );
    eprintln!("{}", ctx.audit.summary());
    Ok(if outcome.cancelled {
        ExitCode::ERROR
    } else {
        ExitCode::OK
    })
}

fn cmd_chat(path: &PathBuf, dry_run: bool, stream: bool) -> Result<ExitCode> {
    let ctx = build_agent_ctx(path, dry_run, stream)?;
    mirror_cancel(Arc::clone(&ctx.cancel));
    run_repl(&ctx)?;
    Ok(ExitCode::OK)
}

fn cmd_tui(
    path: Option<PathBuf>,
    dry_run: bool,
    resume: bool,
    plain: bool,
) -> Result<ExitCode> {
    if plain {
        let path = path.ok_or_else(|| anyhow!("`--plain` требует PATH"))?;
        return cmd_chat(&path, dry_run, true);
    }
    crate::tui::run(crate::tui::TuiOptions {
        path,
        dry_run,
        resume,
    })?;
    Ok(ExitCode::OK)
}

fn cmd_generate(
    path: &PathBuf,
    kind: &str,
    write: bool,
    update: bool,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    let config = load_config(&profile)?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    if plan.items.is_empty() {
        eprintln!("нет целей");
        return Ok(ExitCode::NO_GOALS);
    }
    let llm = need_llm(&config)?;
    let mut report = RunReport {
        warnings: plan.warnings.clone(),
        items: vec![],
        metrics: Default::default(),
    };

    for item in &plan.items {
        match generate_draft(llm.as_ref(), &profile, &config, item) {
            Ok(draft) => match apply_draft(&profile, &draft, write, update) {
                Ok(crate::apply::ApplyOutcome::Diff(diff)) => {
                    if matches!(format, OutputFormat::Text) {
                        println!("{diff}");
                    }
                    report.items.push(crate::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: draft.path,
                        attempts: 0,
                        status: ItemStatus::DryRun,
                        command: String::new(),
                        message: diff,
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
                Ok(crate::apply::ApplyOutcome::Written(p)) => {
                    report.items.push(crate::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: p,
                        attempts: 0,
                        status: ItemStatus::Written,
                        command: String::new(),
                        message: "записан".into(),
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
                Err(err) => {
                    report.items.push(crate::model::ReportItem {
                        symbol: item.gap.symbol.name.clone(),
                        kind: item.kind,
                        file: draft.path,
                        attempts: 0,
                        status: ItemStatus::FailedPath,
                        command: String::new(),
                        message: err.to_string(),
                        trace_path: None,
                        first_pass: None,
                        assert_density: None,
                        usefulness: None,
                    });
                }
            },
            Err(err) => {
                if err.to_string().to_ascii_lowercase().contains("llm")
                    || err.to_string().contains("HTTP")
                    || err.to_string().contains("connect")
                {
                    return Err(err.context("llm недоступна"));
                }
                report.warnings.push(err.to_string());
            }
        }
    }

    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}

fn cmd_run(
    path: &PathBuf,
    kind: &str,
    write: bool,
    update: bool,
    format: OutputFormat,
    opts: PlanOptions,
    package: Option<String>) -> Result<ExitCode> {
    let kind = parse_kind(kind)?;
    let mut profile = prepare_project(path)?;
    if let Some(pkg) = package {
        profile.package_filter = Some(pkg);
    }
    if !profile.supported {
        bail!(
            "{}: {}",
            i18n::t(Msg::UnsupportedLanguage),
            profile.language.as_str()
        );
    }
    let config = load_config(&profile)?;
    apply_config_flags(&mut profile, &config);
    let plan = build_plan_with(&profile, &config, kind, &opts)?;
    if plan.items.is_empty() {
        for w in &plan.warnings {
            eprintln!("{}{w}", i18n::t(Msg::WarningPrefix));
        }
        eprintln!("{}", i18n::t(Msg::NoGoals));
        return Ok(ExitCode::NO_GOALS);
    }
    let llm = need_llm(&config).map_err(|e| e.context(i18n::t(Msg::LlmUnavailable)))?;
    eprintln!(
        "| plan -> generate -> validate -> repair ({})",
        plan.items.len()
    );
    let items = run_plan(llm.as_ref(), &profile, &config, &plan, write, update)?;
    let mut report = RunReport {
        items,
        warnings: plan.warnings,
        metrics: Default::default(),
    };
    crate::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}

fn cmd_repair(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let config = load_config(&profile)?;
    let state = load_state(&profile.root)?;
    let llm = need_llm(&config)?;
    let plan = build_plan(&profile, &config, state.kind)?;
    let item = plan
        .items
        .iter()
        .find(|i| i.gap.symbol.name == state.item_symbol)
        .cloned()
        .ok_or_else(|| anyhow!("элемент плана '{}' не найден для repair", state.item_symbol))?;

    let report_item = {
        apply_draft(&profile, &state.draft, true, true)?;

        use crate::generate::generate_repair;
        use crate::validate::validate_draft;
        let mut draft = state.draft.clone();
        let mut attempts = 0u32;
        let mut last_err = state.error.clone();
        let final_item;
        loop {
            attempts += 1;
            draft = generate_repair(llm.as_ref(), &profile, &item, &draft, &last_err)?;
            apply_draft(&profile, &draft, true, true)?;
            let result = validate_draft(&profile, &config, &draft)?;
            if result.ok {
                let _ = std::fs::remove_file(crate::repair::state_path(&profile.root));
                final_item = crate::model::ReportItem {
                    symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                    file: draft.path.clone(),
                    attempts,
                    status: ItemStatus::Passed,
                    command: result.command,
                    message: "пройден".into(),
                    trace_path: result.trace_path,
                    first_pass: None,
                    assert_density: None,
                    usefulness: None,
                };
                break;
            }
            last_err = format!("{}\n{}", result.stdout, result.stderr);
            save_state(
                &profile.root,
                &LastRepairState {
                    root: profile.root.clone(),
                    draft: draft.clone(),
                    error: last_err.clone(),
                    item_symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                },
            )?;
            if attempts >= config.run.repair_attempts {
                final_item = crate::model::ReportItem {
                    symbol: item.gap.symbol.name.clone(),
                    kind: item.kind,
                    file: draft.path.clone(),
                    attempts,
                    status: ItemStatus::Unfixed,
                    command: result.command,
                    message: last_err.chars().take(2000).collect(),
                    trace_path: result.trace_path,
                    first_pass: None,
                    assert_density: None,
                    usefulness: None,
                };
                break;
            }
        }
        final_item
    };

    let report = RunReport {
        items: vec![report_item],
        warnings: vec![],
        metrics: Default::default(),
    };
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}

fn cmd_ui_init(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = discover(path)?;
    let root = profile.root.clone();

    let config_path = root.join("playwright.config.ts");
    if !config_path.exists() {
        let cfg = r#"import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL: 'http://127.0.0.1:3456',
    trace: 'retain-on-failure',
  },
});
"#;
        write_bytes(&config_path, cfg.as_bytes())?;
    }

    let e2e = root.join("e2e");
    std::fs::create_dir_all(&e2e)?;
    let gitkeep = e2e.join(".gitkeep");
    if !gitkeep.exists() {
        write_bytes(&gitkeep, b"")?;
    }

    let pkg_path = root.join("package.json");
    if pkg_path.exists() {
        let text = std::fs::read_to_string(&pkg_path)?;
        let mut pkg: serde_json::Value = serde_json::from_str(&text)?;
        let dev = pkg
            .as_object_mut()
            .unwrap()
            .entry("devDependencies")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(obj) = dev.as_object_mut() {
            obj.entry("@playwright/test".to_string())
                .or_insert_with(|| serde_json::json!("^1.48.0"));
        }
        std::fs::write(&pkg_path, serde_json::to_string_pretty(&pkg)?)?;
    }

    let tester_toml = root.join("tester.toml");
    if !tester_toml.exists() {
        let body = r#"[tests.ui]
framework = "playwright"
base_url = "http://127.0.0.1:3456"
start = "python3 -m http.server 3456"

[run]
repair_attempts = 3
timeout_secs = 180
"#;
        write_bytes(&tester_toml, body.as_bytes())?;
    }

    let status = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&root)
        .status();
    if let Ok(st) = status
        && st.success()
    {
        let _ = std::process::Command::new("npx")
            .args(["playwright", "install", "chromium"])
            .current_dir(&root)
            .status();
    }

    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "playwright_config": config_path,
                "e2e": e2e,
            })
        ),
        OutputFormat::Text => {
            println!("{} {}", i18n::t(Msg::PlaywrightInit), root.display());
            println!("{}: {}", i18n::t(Msg::ConfigLabel), config_path.display());
            println!("{}: {}", i18n::t(Msg::TestsLabel), e2e.display());
        }
    }
    Ok(ExitCode::OK)
}

fn cmd_plan_show(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let plan = crate::plan_store::load_plan(&profile.root)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        OutputFormat::Text => {
            for item in &plan.items {
                println!(
                    "- [{}] {} -> {}",
                    item.kind.as_str(),
                    item.gap.symbol.name,
                    item.target_path.display()
                );
            }
            if plan.items.is_empty() {
                println!("нет целей в .tester/plan.json");
            }
        }
    }
    Ok(ExitCode(if plan.items.is_empty() { 2 } else { 0 }))
}

fn cmd_plan_apply(
    path: &PathBuf,
    write: bool,
    update: bool,
    format: OutputFormat) -> Result<ExitCode> {
    let profile = prepare_project(path)?;
    let config = load_config(&profile)?;
    let plan = crate::plan_store::load_plan(&profile.root)?;
    if plan.items.is_empty() {
        return Ok(ExitCode::NO_GOALS);
    }
    let llm = need_llm(&config)?;
    let items = run_plan(llm.as_ref(), &profile, &config, &plan, write, update)?;
    let mut report = RunReport {
        items,
        warnings: plan.warnings,
        metrics: Default::default(),
    };
    crate::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}

fn cmd_eval(
    path: &PathBuf,
    max_symbols: usize,
    suite: bool,
    as_json: bool,
    format: OutputFormat,
) -> Result<ExitCode> {
    if suite {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let root = if path.join("tests/fixtures").is_dir() {
            path.join("tests/fixtures")
        } else if path.file_name().and_then(|s| s.to_str()) == Some("fixtures") {
            path.clone()
        } else {
            fixtures
        };
        let text = eval_suite(&root)?;
        println!("{text}");
        return Ok(ExitCode::OK);
    }
    let report = eval_report(path, max_symbols, true)?;
    if as_json || matches!(format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render_eval(&report));
    }
    Ok(ExitCode::OK)
}

fn cmd_debug(
    path: &PathBuf,
    bug: Option<String>,
    bug_file: Option<&std::path::Path>,
    stdin: bool,
    reproduce_only: bool,
    dry_run: bool,
    update: bool,
    kind: &str,
    max_items: usize,
    format: OutputFormat) -> Result<ExitCode> {
    let bug_text = crate::debug::read_bug_input(bug, bug_file, stdin)?;
    let mut profile = prepare_project(path)?;
    let config = load_config(&profile)?;
    apply_config_flags(&mut profile, &config);
    let llm = need_llm(&config)?;
    let opts = crate::debug::DebugOptions {
        bug_text,
        reproduce_only,
        write: !dry_run,
        update,
        kind: parse_kind(kind)?,
        max_items,
    };
    let mut report = crate::debug::run_debug(llm.as_ref(), &profile, &config, &opts)?;
    crate::repair::enrich_report(&mut report);
    emit_report(&profile, &report, format)?;
    Ok(ExitCode(exit_for(&report) as u8))
}

fn cmd_doctor(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let report = crate::doctor::run_doctor(path)?;
    match format {
        OutputFormat::Json => println!("{}", report.render_json()),
        OutputFormat::Text => print!("{}", report.render_text()),
    }
    Ok(if report.all_ok() {
        ExitCode::OK
    } else {
        ExitCode::ERROR
    })
}

fn cmd_ui_doctor(path: &PathBuf, format: OutputFormat) -> Result<ExitCode> {
    let report = crate::doctor::run_ui_doctor(path)?;
    match format {
        OutputFormat::Json => println!("{}", report.render_json()),
        OutputFormat::Text => print!("{}", report.render_text()),
    }
    Ok(if report.all_ok() {
        ExitCode::OK
    } else {
        ExitCode::ERROR
    })
}

fn emit_report(
    profile: &crate::model::ProjectProfile,
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
