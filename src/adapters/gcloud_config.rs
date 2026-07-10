//! Read-only access to gcloud's configuration state on disk.
//! Formats are gcloud's own and unstable: parse defensively
//! (rules/gcloud-safety.md).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use crate::core::error::ConfigError;

/// Environment variable gcloud itself honors to relocate its config directory.
pub const CLOUDSDK_CONFIG_ENV: &str = "CLOUDSDK_CONFIG";

/// Resolve the gcloud configuration directory for this platform.
/// The single source of truth for this path; nothing else may construct it.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    resolve_config_dir(env::var_os(CLOUDSDK_CONFIG_ENV))
}

fn resolve_config_dir(override_var: Option<OsString>) -> Result<PathBuf, ConfigError> {
    match override_var {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => platform_default_dir(),
    }
}

// gcloud uses %APPDATA%\gcloud on Windows, not a home-relative path.
#[cfg(windows)]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::var_os("APPDATA")
        .filter(|appdata| !appdata.is_empty())
        .map(|appdata| PathBuf::from(appdata).join("gcloud"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

// gcloud uses ~/.config/gcloud on both Linux and macOS (it does not follow
// the macOS ~/Library convention).
#[cfg(not(windows))]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::home_dir()
        .map(|home| home.join(".config").join("gcloud"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

/// Name of the currently active gcloud configuration.
///
/// gcloud stores it as the plain-text content of `active_config` and falls
/// back to "default" when the file is absent; so do we.
pub fn active_config_name(config_dir: &Path) -> Result<String, ConfigError> {
    let path = config_dir.join("active_config");
    match fs::read_to_string(&path) {
        Ok(name) => Ok(name.trim().to_string()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok("default".to_string()),
        Err(source) => Err(ConfigError::Unreadable { path, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-test scratch directory; std-only stand-in for tempfile.
    fn scratch_dir(test: &str) -> PathBuf {
        let dir = env::temp_dir()
            .join("hop-tests")
            .join(format!("{test}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("failed to create scratch dir");
        dir
    }

    #[test]
    fn override_env_wins_over_platform_default() {
        // arrange
        let override_var = Some(OsString::from("/custom/gcloud-config"));
        // act
        let dir = resolve_config_dir(override_var).expect("resolution failed");
        // assert
        assert_eq!(dir, PathBuf::from("/custom/gcloud-config"));
    }

    #[test]
    fn empty_override_falls_back_to_platform_default() {
        // arrange
        let override_var = Some(OsString::new());
        // act
        let dir = resolve_config_dir(override_var).expect("resolution failed");
        // assert
        assert!(dir.ends_with("gcloud"), "unexpected dir: {}", dir.display());
    }

    #[test]
    fn active_config_name_reads_and_trims_the_file() {
        // arrange
        let dir = scratch_dir("active-config-present");
        fs::write(dir.join("active_config"), "work\n").expect("write failed");
        // act
        let name = active_config_name(&dir).expect("read failed");
        // assert
        assert_eq!(name, "work");
    }

    #[test]
    fn active_config_name_defaults_when_file_is_absent() {
        // arrange
        let dir = scratch_dir("active-config-absent");
        // act
        let name = active_config_name(&dir).expect("read failed");
        // assert
        assert_eq!(name, "default");
    }
}
