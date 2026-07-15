//! Ports: traits describing what core needs from the outside world.
//! Adapters implement them; core never knows which one (rules/architecture.md).

use std::path::Path;

use crate::core::context::{Configuration, Context, Project, ServiceAccountInfo};
use crate::core::error::{ApiError, AuthError, BrowserError, ConfigError, PromptError};
use crate::core::settings::Settings;
use crate::core::types::{AccessToken, AccountEmail, ProjectId, ServiceAccount};

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
    /// Set or clear (`None`) impersonation on the named configuration.
    fn set_impersonation(
        &self,
        name: &str,
        service_account: Option<&ServiceAccount>,
    ) -> Result<(), ConfigError>;
}

/// Lets the user choose a configuration interactively.
pub trait ConfigurationPicker {
    /// Present the choices under `prompt`; `Ok(None)` means the user
    /// cancelled. `prompt` lets callers phrase the action (switch vs open).
    fn pick(
        &self,
        prompt: &str,
        configurations: &[Configuration],
    ) -> Result<Option<String>, PromptError>;
}

/// Lets the user choose a project interactively.
pub trait ProjectPicker {
    /// Whether picking can happen at all (e.g. a terminal is attached).
    /// Callers use this to skip expensive work when nothing can be picked.
    fn available(&self) -> bool {
        true
    }
    /// Present the choices under `prompt`; `Ok(None)` means the user
    /// cancelled. `prompt` lets callers phrase the action (switch vs open).
    fn pick(&self, prompt: &str, projects: &[Project]) -> Result<Option<ProjectId>, PromptError>;
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
    /// `login_config` selects the workforce (SSO) flow when present.
    fn login(
        &self,
        account: Option<&AccountEmail>,
        no_launch_browser: bool,
        login_config: Option<&Path>,
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

/// Lists the service accounts of a project (for the impersonation picker).
pub trait ServiceAccountLister {
    /// Enabled service accounts in `project`, sorted by email.
    fn list_service_accounts(
        &self,
        token: &AccessToken,
        project: &ProjectId,
    ) -> Result<Vec<ServiceAccountInfo>, ApiError>;
}

/// Proves impersonation works before hop commits it to config.
pub trait ImpersonationVerifier {
    /// Mint (and immediately discard) a short-lived token as
    /// `service_account`. Success means the caller holds
    /// `iam.serviceAccounts.getAccessToken` on it.
    fn verify_impersonation(
        &self,
        token: &AccessToken,
        service_account: &ServiceAccount,
    ) -> Result<(), ApiError>;
}

/// Lets the user choose a service account interactively.
pub trait ServiceAccountPicker {
    /// Present the choices; `Ok(None)` means the user cancelled.
    fn pick(&self, accounts: &[ServiceAccountInfo]) -> Result<Option<ServiceAccount>, PromptError>;
}

/// Opens a URL in the user's browser.
pub trait BrowserOpener {
    /// Hand `url` to the system's URL handler.
    fn open_url(&self, url: &str) -> Result<(), BrowserError>;
}
