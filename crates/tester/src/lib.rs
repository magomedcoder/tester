#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::collapsible_if
)]

pub mod cli;

pub use cli::run;
