//! hop's own user settings (not gcloud state).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// What to do when a switch needs credentials that turn out to be expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReauthPolicy {
    /// Ask before launching the login flow (the granted.dev-style default).
    #[default]
    Prompt,
    /// Launch the login flow immediately without asking.
    Auto,
    /// Never launch the login flow; fail and let the user run `hop login`.
    Off,
}

/// All of hop's settings, with defaults for anything unset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    /// Re-authentication behaviour on expired credentials.
    pub reauth: ReauthPolicy,
    /// Browser command for login flows and `hop console`; `None` means the
    /// system default. Invoked with the URL as its single argument.
    pub browser: Option<PathBuf>,
}

/// Resolve which browser command applies: an explicit `BROWSER` environment
/// variable (passed in by the caller; core reads no env itself) wins over
/// the configured setting, and an empty variable counts as unset.
pub fn effective_browser(env_value: Option<&OsStr>, configured: Option<&Path>) -> Option<PathBuf> {
    match env_value {
        Some(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => configured.map(Path::to_path_buf),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn env_variable_wins_over_the_setting() {
        // arrange
        let env = OsString::from("/usr/bin/env-browser");
        let configured = PathBuf::from("/usr/bin/configured-browser");
        // act
        let chosen = effective_browser(Some(&env), Some(&configured));
        // assert
        assert_eq!(chosen, Some(PathBuf::from("/usr/bin/env-browser")));
    }

    #[test]
    fn empty_env_variable_falls_back_to_the_setting() {
        // arrange
        let env = OsString::new();
        let configured = PathBuf::from("/usr/bin/configured-browser");
        // act
        let chosen = effective_browser(Some(&env), Some(&configured));
        // assert
        assert_eq!(chosen, Some(configured));
    }

    #[test]
    fn setting_applies_when_no_env_variable_is_set() {
        // arrange
        let configured = PathBuf::from("/usr/bin/configured-browser");
        // act
        let chosen = effective_browser(None, Some(&configured));
        // assert
        assert_eq!(chosen, Some(configured));
    }

    #[test]
    fn neither_source_means_the_system_default() {
        // arrange: nothing set anywhere
        // act
        let chosen = effective_browser(None, None);
        // assert
        assert_eq!(chosen, None);
    }
}
