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
        command.args(login_args(account, no_launch_browser, login_config));
        // gcloud launches the browser through Python's webbrowser module,
        // which honours BROWSER; an explicit variable in hop's own
        // environment wins over the settings value (re-setting it to the
        // inherited value is a no-op).
        if let Some(browser) =
            effective_browser(env::var_os("BROWSER").as_deref(), self.browser.as_deref())
        {
            command.env("BROWSER", browser);
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

/// Build the `gcloud auth login` argument vector.
///
/// The ACCOUNT positional is omitted whenever a login config is present: a
/// workforce (SSO) sign-in is federated and driven by `--login-config`, and
/// gcloud treats a positional ACCOUNT as "reuse the stored credential if it
/// exists". Passing the `principal://...` subject there makes gcloud try to
/// *refresh* the expired workforce token (failing with `invalid_grant`)
/// instead of signing in fresh. The account is only meaningful for the plain
/// Google OAuth flow.
fn login_args(
    account: Option<&AccountEmail>,
    no_launch_browser: bool,
    login_config: Option<&Path>,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["auth".into(), "login".into(), "--brief".into()];
    if no_launch_browser {
        args.push("--no-launch-browser".into());
    }
    match login_config {
        Some(path) => {
            // Built as one OsString so non-UTF-8 paths survive intact.
            let mut flag = OsString::from("--login-config=");
            flag.push(path);
            args.push(flag);
        }
        None => {
            if let Some(account) = account {
                args.push(account.as_str().into());
            }
        }
    }
    args
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

#[cfg(test)]
mod tests {
    use super::*;

    fn as_strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn google_flow_passes_the_account() {
        // arrange
        let account = AccountEmail::new("dev@example.com").expect("valid");
        // act
        let args = as_strings(&login_args(Some(&account), false, None));
        // assert
        assert_eq!(args, ["auth", "login", "--brief", "dev@example.com"]);
    }

    #[test]
    fn sso_flow_omits_the_account_positional() {
        // arrange: a workforce principal that must never reach gcloud as ACCOUNT
        let account =
            AccountEmail::new("principal://iam.googleapis.com/.../subject/adm-1@example.cloud")
                .expect("valid");
        let config = Path::new("/home/dev/wf-login.json");
        // act
        let args = as_strings(&login_args(Some(&account), false, Some(config)));
        // assert: login config drives the flow; the principal is not present
        assert_eq!(
            args,
            [
                "auth",
                "login",
                "--brief",
                "--login-config=/home/dev/wf-login.json"
            ]
        );
        assert!(!args.iter().any(|a| a.starts_with("principal://")));
    }

    #[test]
    fn no_launch_browser_adds_the_flag() {
        // act
        let args = as_strings(&login_args(None, true, None));
        // assert
        assert_eq!(args, ["auth", "login", "--brief", "--no-launch-browser"]);
    }

    #[test]
    fn google_flow_without_an_account_passes_no_positional() {
        // act
        let args = as_strings(&login_args(None, false, None));
        // assert
        assert_eq!(args, ["auth", "login", "--brief"]);
    }
}
