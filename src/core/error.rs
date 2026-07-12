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
}
