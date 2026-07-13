//! Spawning the gcloud binary: the only place hop executes external
//! processes. Per the hybrid backend decision, gcloud is delegated to for
//! login flows and token minting only; arguments are always passed as
//! arrays, never concatenated into shell strings (rules/security.md).

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::core::error::AuthError;
use crate::core::ports::{Authenticator, TokenProvider};
use crate::core::settings::effective_browser;
use crate::core::types::{AccessToken, AccountEmail};

// Windows installs gcloud as a .cmd shim, which Command::new("gcloud")
// does not resolve; the binary name is therefore cfg-gated.
#[cfg(windows)]
const GCLOUD: &str = "gcloud.cmd";
#[cfg(not(windows))]
const GCLOUD: &str = "gcloud";

/// The gcloud CLI as an auth backend.
pub struct GcloudCli {
    /// Browser command for login flows; `None` leaves the choice to gcloud.
    browser: Option<PathBuf>,
}

impl GcloudCli {
    /// gcloud with an optional browser command (the `browser` setting).
    pub fn new(browser: Option<PathBuf>) -> Self {
        Self { browser }
    }
}

impl Authenticator for GcloudCli {
    fn login(
        &self,
        account: Option<&AccountEmail>,
        no_launch_browser: bool,
        login_config: Option<&Path>,
    ) -> Result<(), AuthError> {
        let mut command = Command::new(GCLOUD);
        command.args(["auth", "login", "--brief"]);
        // gcloud launches the browser through Python's webbrowser module,
        // which honours BROWSER; an explicit variable in hop's own
        // environment wins over the settings value (re-setting it to the
        // inherited value is a no-op).
        if let Some(browser) =
            effective_browser(env::var_os("BROWSER").as_deref(), self.browser.as_deref())
        {
            command.env("BROWSER", browser);
        }
        if no_launch_browser {
            command.arg("--no-launch-browser");
        }
        if let Some(path) = login_config {
            // Built as one OsString so non-UTF-8 paths survive intact.
            let mut flag = OsString::from("--login-config=");
            flag.push(path);
            command.arg(flag);
        }
        if let Some(account) = account {
            command.arg(account.as_str());
        }
        // Inherited stdio: gcloud owns the browser flow and its prompts.
        let status = command
            .status()
            .map_err(|err| AuthError::GcloudUnavailable {
                detail: err.to_string(),
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(AuthError::LoginFailed)
        }
    }
}

impl TokenProvider for GcloudCli {
    fn access_token(&self, account: &AccountEmail) -> Result<AccessToken, AuthError> {
        let output = Command::new(GCLOUD)
            .args(["auth", "print-access-token", account.as_str()])
            .stdin(Stdio::null())
            .output()
            .map_err(|err| AuthError::GcloudUnavailable {
                detail: err.to_string(),
            })?;
        if !output.status.success() {
            // gcloud documents no exit codes for expired/revoked credentials,
            // so any failure counts as needing re-auth; its stderr carries
            // the human-readable reason (never the token).
            return Err(AuthError::CredentialsInvalid {
                account: account.as_str().to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        AccessToken::new(String::from_utf8_lossy(&output.stdout).as_ref())
            .map_err(|_| AuthError::EmptyToken)
    }
}
