//! Orchestration layer: the composition root where core meets adapters,
//! and the owner of process exit codes (rules/architecture.md).

mod console;
mod impersonate;
mod login;
mod status;
mod switch;

use std::process::ExitCode;

use crate::cli::{Cli, Command};

/// Running `hop` with no subcommand defaults to the interactive switcher.
pub fn run(cli: Cli) -> ExitCode {
    match cli.command.unwrap_or(Command::Switch) {
        Command::Login { account } => login::run(account.as_deref()),
        Command::Switch => switch::run(),
        Command::Console { project } => console::run(project.as_deref()),
        Command::Impersonate {
            service_account,
            clear,
        } => impersonate::run(service_account.as_deref(), clear),
        Command::Status => status::run(),
    }
}

/// Shared stub for commands whose implementation lands in a later increment.
fn not_implemented(command: &str, phase: &str) -> ExitCode {
    eprintln!("hop {command}: not implemented yet (arrives in {phase})");
    ExitCode::FAILURE
}
