use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "tester",
    version,
    about = "LLM agent for generating and fixing tests",
    subcommand_negates_reqs = true
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[arg(long, global = true)]
    pub debug: bool,

    #[arg(long, global = true, value_name = "LOCALE")]
    pub locale: Option<String>,

    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long = "session", value_name = "ID")]
    pub session: Option<String>,

    #[arg(long)]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Test(TestCommands),

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

    Sessions {
        #[command(subcommand)]
        action: SessionsCmd,
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
pub enum TestCommands {
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
}

#[derive(Debug, Subcommand)]
pub enum SessionsCmd {
    List {
        #[arg(long, value_name = "PATH")]
        workspace: Option<PathBuf>,
    },
    Prune {
        #[arg(value_name = "ID")]
        id: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long, value_name = "PATH")]
        workspace: Option<PathBuf>,
        #[arg(long, value_name = "DAYS")]
        older_than: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
pub enum UiCommands {
    Init { path: PathBuf },
    Doctor { path: PathBuf },
}
