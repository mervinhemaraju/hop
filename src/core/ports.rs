//! Ports: traits describing what core needs from the outside world.
//! Adapters implement them; core never knows which one (rules/architecture.md).

use crate::core::context::{Configuration, Context, Project};
use crate::core::error::{ApiError, AuthError, ConfigError, PromptError};
use crate::core::settings::Settings;
use crate::core::types::{AccessToken, AccountEmail, ProjectId};

/// Provides the currently active GCP context.
///
/// Implemented by the gcloud config-file adapter in production and by plain
/// fakes in tests.
pub trait ContextSource {
    /// Read the active context.
    fn active_context(&self) -> Result<Context, ConfigError>;
}

/// Lists gcloud configurations and switches the active one.
pub trait ConfigurationStore {
    /// All configurations, sorted by name.
    fn list(&self) -> Result<Vec<Configuration>, ConfigError>;
    /// Make `name` the active configuration; fails if it does not exist.
    fn activate(&self, name: &str) -> Result<(), ConfigError>;
    /// Set the project property on the named configuration.
    fn set_project(&self, name: &str, project: &ProjectId) -> Result<(), ConfigError>;
}

/// Lets the user choose a configuration interactively.
pub trait ConfigurationPicker {
    /// Present the choices; `Ok(None)` means the user cancelled.
    fn pick(&self, configurations: &[Configuration]) -> Result<Option<String>, PromptError>;
}

/// Lets the user choose a project interactively.
pub trait ProjectPicker {
    /// Whether picking can happen at all (e.g. a terminal is attached).
    /// Callers use this to skip expensive work when nothing can be picked.
    fn available(&self) -> bool {
        true
    }
    /// Present the choices; `Ok(None)` means the user cancelled.
    fn pick(&self, projects: &[Project]) -> Result<Option<ProjectId>, PromptError>;
}

/// Asks the user a yes/no question.
pub trait Confirmer {
    /// `Ok(None)` means the user cancelled rather than answering.
    fn confirm(&self, message: &str) -> Result<Option<bool>, PromptError>;
}

/// Runs the interactive gcloud login flow.
pub trait Authenticator {
    /// Log in, optionally to a specific account. `no_launch_browser` follows
    /// gcloud's flag of the same name for remote/SSH sessions.
    fn login(
        &self,
        account: Option<&AccountEmail>,
        no_launch_browser: bool,
    ) -> Result<(), AuthError>;
}

/// Mints short-lived access tokens for an authenticated account.
pub trait TokenProvider {
    /// A token for `account`; failure means the credentials need re-auth.
    fn access_token(&self, account: &AccountEmail) -> Result<AccessToken, AuthError>;
}

/// Lists the projects the token's identity can see.
pub trait ProjectLister {
    /// Active projects, sorted by id.
    fn list_projects(&self, token: &AccessToken) -> Result<Vec<Project>, ApiError>;
}

/// Local cache of project listings, keyed by account.
pub trait ProjectCache {
    /// Cached projects for `account`, or `None` when nothing is cached.
    fn cached_projects(&self, account: &AccountEmail) -> Result<Option<Vec<Project>>, ConfigError>;
    /// Replace the cache for `account`.
    fn store_projects(
        &self,
        account: &AccountEmail,
        projects: &[Project],
    ) -> Result<(), ConfigError>;
}

/// Reads hop's own settings.
pub trait SettingsStore {
    /// Current settings, with defaults where the file is absent or silent.
    fn settings(&self) -> Result<Settings, ConfigError>;
}
