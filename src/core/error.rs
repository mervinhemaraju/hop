use std::path::PathBuf;

use thiserror::Error;

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
