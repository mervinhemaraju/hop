use std::process::ExitCode;

use crate::adapters::gcloud_config;

/// Show the active gcloud context. This increment shows the configuration
/// name only; account, project, and impersonation details arrive with the
/// config parser in Phase 2.
pub fn run() -> ExitCode {
    let dir = match gcloud_config::config_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("hop status: {err}");
            return ExitCode::FAILURE;
        }
    };
    match gcloud_config::active_config_name(&dir) {
        Ok(name) => {
            eprintln!("config directory:     {}", dir.display());
            eprintln!("active configuration: {name}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("hop status: {err}");
            ExitCode::FAILURE
        }
    }
}
