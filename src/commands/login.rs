use std::path::PathBuf;
use std::process::ExitCode;

use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::gcloud_process::GcloudCli;
use crate::adapters::hop_files::HopFiles;
use crate::commands::EXIT_BAD_INPUT;
use crate::core::ports::{Authenticator, ContextSource, SettingsStore};
use crate::core::types::AccountEmail;

/// Launch gcloud's login flow: the browser OAuth flow for Google accounts,
/// or the workforce (SSO) flow when a login config is given or configured.
pub fn run(
    account: Option<&str>,
    sso: bool,
    login_config: Option<&str>,
    no_launch_browser: bool,
) -> ExitCode {
    let account = match account.map(AccountEmail::new).transpose() {
        Ok(account) => account,
        Err(err) => {
            eprintln!("hop login: {err}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };
    let login_config = match resolve_login_config(sso, login_config) {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    // The browser setting rides along so gcloud opens the one the user chose.
    let settings = match HopFiles::new().and_then(|files| files.settings()) {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("hop login: {err}");
            return ExitCode::FAILURE;
        }
    };
    // Composition root: gcloud is the only login backend (hybrid decision).
    let gcloud = GcloudCli::new(settings.browser);
    match gcloud.login(account.as_ref(), no_launch_browser, login_config.as_deref()) {
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

// Resolve which login config (if any) to hand to gcloud:
// --login-config wins; --sso alone reads the active configuration's
// auth/login_config_file property; neither means the plain Google flow
// (where gcloud still applies the property itself if the user set it).
fn resolve_login_config(
    sso: bool,
    login_config: Option<&str>,
) -> Result<Option<PathBuf>, ExitCode> {
    if let Some(raw) = login_config {
        let path = PathBuf::from(raw);
        if !path.is_file() {
            eprintln!(
                "hop login: login config file not found: {raw}; create one with `gcloud iam workforce-pools create-login-config`"
            );
            return Err(ExitCode::from(EXIT_BAD_INPUT));
        }
        return Ok(Some(path));
    }
    if !sso {
        return Ok(None);
    }
    let context = GcloudConfigSource::new()
        .and_then(|source| source.active_context())
        .map_err(|err| {
            eprintln!("hop login: {err}");
            ExitCode::FAILURE
        })?;
    match context.login_config_file {
        Some(raw) => {
            let path = PathBuf::from(&raw);
            if !path.is_file() {
                eprintln!(
                    "hop login: the configured login config does not exist: {raw}; re-run `gcloud iam workforce-pools create-login-config <provider> --activate` or pass --login-config"
                );
                return Err(ExitCode::FAILURE);
            }
            Ok(Some(path))
        }
        None => {
            eprintln!(
                "hop login: --sso needs a workforce login config, but the active configuration has none; pass --login-config <file> or run `gcloud iam workforce-pools create-login-config <provider> --activate` first"
            );
            Err(ExitCode::FAILURE)
        }
    }
}
