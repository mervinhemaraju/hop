use std::process::ExitCode;

use crate::adapters::gcloud_process::GcloudCli;
use crate::commands::EXIT_BAD_INPUT;
use crate::core::ports::Authenticator;
use crate::core::types::AccountEmail;

/// Launch gcloud's browser-based login flow.
pub fn run(account: Option<&str>, no_launch_browser: bool) -> ExitCode {
    let account = match account.map(AccountEmail::new).transpose() {
        Ok(account) => account,
        Err(err) => {
            eprintln!("hop login: {err}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };
    // Composition root: gcloud is the only login backend (hybrid decision).
    match GcloudCli.login(account.as_ref(), no_launch_browser) {
        Ok(()) => {
            eprintln!("login complete");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("hop login: {err}");
            ExitCode::FAILURE
        }
    }
}
