//! Spawning the gcloud binary: the only place hop executes external
//! processes. Per the hybrid backend decision, gcloud is delegated to for
//! login flows and token minting only; arguments are always passed as
//! arrays, never concatenated into shell strings (rules/security.md).

use std::process::{Command, Stdio};

use crate::core::error::AuthError;
use crate::core::ports::{Authenticator, TokenProvider};
use crate::core::types::{AccessToken, AccountEmail};

// Windows installs gcloud as a .cmd shim, which Command::new("gcloud")
// does not resolve; the binary name is therefore cfg-gated.
#[cfg(windows)]
const GCLOUD: &str = "gcloud.cmd";
#[cfg(not(windows))]
const GCLOUD: &str = "gcloud";

/// The gcloud CLI as an auth backend.
pub struct GcloudCli;

impl Authenticator for GcloudCli {
    fn login(
        &self,
        account: Option<&AccountEmail>,
        no_launch_browser: bool,
    ) -> Result<(), AuthError> {
        let mut command = Command::new(GCLOUD);
        command.args(["auth", "login", "--brief"]);
        if no_launch_browser {
            command.arg("--no-launch-browser");
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
