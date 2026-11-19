#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
)]

pub mod adapter;
pub mod agent;
pub mod api_detect;
pub mod apply;
pub mod cache;
pub mod cli;
pub mod config;
pub mod coverage;
pub mod debug;
pub mod discover;
pub mod doctor;
pub mod error;
pub mod eval;
pub mod failure;
pub mod gate;
pub mod generate;
pub mod green_cache;
pub mod index;
pub mod inventory;
pub mod llm;
pub mod model;
pub mod plan;
pub mod plan_store;
pub mod prompts;
pub mod repair;
pub mod report;
pub mod security;
pub mod validate;

pub use cli::run;
pub use error::ExitCode;
