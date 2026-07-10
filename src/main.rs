mod adapters;
mod cli;
mod commands;
mod core;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    commands::run(cli::Cli::parse())
}
