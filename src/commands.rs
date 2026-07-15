//! Orchestration layer: the composition root where core meets adapters,
//! and the owner of process exit codes (rules/architecture.md).

mod auth_flow;
mod console;
mod impersonate;
mod login;
mod project_source;
mod status;
mod switch;

use std::process::ExitCode;

use crate::cli::{Cli, Command};

// Exit code classes (rules/cli-ux.md): distinct codes per failure class so
// scripts can react. 0 = success, 1 = general failure. clap itself exits 2
// on usage errors, so bad input shares that class.
/// Bad input, e.g. a configuration name that does not exist.
pub const EXIT_BAD_INPUT: u8 = 2;
/// An interactive prompt was needed but no TTY is available.
pub const EXIT_NOT_INTERACTIVE: u8 = 3;
/// Credentials are expired or revoked and no re-auth happened.
pub const EXIT_NOT_AUTHENTICATED: u8 = 4;
/// Authenticated, but lacking permission (e.g. no token-creator role).
pub const EXIT_PERMISSION_DENIED: u8 = 5;
/// The user cancelled an interactive prompt (128 + SIGINT by convention).
pub const EXIT_CANCELLED: u8 = 130;

/// Running `hop` with no subcommand defaults to the interactive switcher.
pub fn run(cli: Cli) -> ExitCode {
    match cli.command.unwrap_or(Command::Switch {
        name: None,
        project: None,
        refresh: false,
    }) {
        Command::Login {
            account,
            sso,
            login_config,
            no_launch_browser,
        } => login::run(
            account.as_deref(),
            sso,
            login_config.as_deref(),
            no_launch_browser,
        ),
        Command::Switch {
            name,
            project,
            refresh,
        } => switch::run(name.as_deref(), project.as_deref(), refresh),
        Command::Console {
            name,
            project,
            url,
            refresh,
        } => console::run(name.as_deref(), project.as_deref(), url, refresh),
        Command::Impersonate {
            service_account,
            clear,
        } => impersonate::run(service_account.as_deref(), clear),
        Command::Status => status::run(),
    }
}
