use std::path::PathBuf;

use thiserror::Error;

/// Rejections raised while constructing validated domain identifiers.
/// `PartialEq` is derivable here (unlike `ConfigError`, which holds
/// `io::Error`) and lets tests assert on exact error values.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("{what} must not be empty; provide a value")]
    Empty { what: &'static str },
    #[error(
        "{what} contains an unsupported character ({found:?}); use only visible ASCII characters with no spaces"
    )]
    InvalidCharacter { what: &'static str, found: char },
}

/// Failures while locating or reading gcloud's local configuration state.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "could not determine the home directory; set CLOUDSDK_CONFIG to your gcloud config directory"
    )]
    HomeDirUnavailable,
    #[error("failed to read {path}: {source}")]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "failed to parse {path}: {detail}; fix the file or recreate it with `gcloud config configurations create`"
    )]
    Malformed { path: PathBuf, detail: String },
    #[error("invalid {property} in {path}: {source}")]
    InvalidProperty {
        path: PathBuf,
        property: String,
        source: ValidationError,
    },
    #[error("failed to write {path}: {source}; the previous state was left untouched")]
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "no configuration named {name:?}; run `hop switch` to pick from the list or create it with `gcloud config configurations create {name}`"
    )]
    UnknownConfiguration { name: String },
    #[error("no gcloud configurations found; run `gcloud init` to create one first")]
    NoConfigurations,
}

/// Failures while authenticating or acquiring tokens through gcloud.
/// Never contains token material; only gcloud's own diagnostics.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error(
        "could not run gcloud: {detail}; install the Google Cloud SDK and make sure `gcloud` is on your PATH"
    )]
    GcloudUnavailable { detail: String },
    #[error(
        "credentials for {account} are expired or revoked ({detail}); run `hop login {account}` to re-authenticate"
    )]
    CredentialsInvalid { account: String, detail: String },
    #[error("gcloud login did not complete; run `hop login` to try again")]
    LoginFailed,
    #[error("gcloud returned an empty access token; try `gcloud auth login` manually")]
    EmptyToken,
}

/// Failures calling GCP APIs. Never contains token material.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("GCP API returned HTTP {0}; if this persists, re-authenticate with `hop login`")]
    Status(u16),
    #[error("network error calling GCP: {0}; check your connection and try again")]
    Network(String),
    #[error(
        "could not decode the GCP response: {0}; the API may have changed, check for a hop update"
    )]
    Decode(String),
}

/// Failure to hand a URL to the system browser.
#[derive(Debug, Error)]
#[error("could not open the browser: {detail}; open the URL printed above manually")]
pub struct BrowserError {
    pub detail: String,
}

/// Failures while running an interactive prompt. User cancellation is not an
/// error (pickers return `Ok(None)` for it); these are real failures.
#[derive(Debug, Error)]
pub enum PromptError {
    #[error(
        "this command needs a terminal for its interactive prompt; pass the target directly, e.g. `hop switch my-config`"
    )]
    NotInteractive,
    #[error("interactive prompt failed: {0}")]
    Backend(String),
}
