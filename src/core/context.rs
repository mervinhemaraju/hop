//! The active GCP context as hop understands it: which gcloud
//! configuration is active and what it binds.

use crate::core::types::{AccountEmail, ProjectId, ServiceAccount};
use crate::core::workforce::PRINCIPAL_PREFIX;

/// How the active identity authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKind {
    /// A plain Google account (browser OAuth flow).
    Google,
    /// Workforce identity federation (SSO through an external IdP).
    Workforce,
}

/// A snapshot of the active gcloud context.
///
/// The optional fields mirror gcloud itself: a configuration always has a
/// name, but may have no account, project, or impersonation set. Until the
/// Phase 2 configuration-file parser lands, adapters fill only `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// Name of the active gcloud configuration (e.g. `default`).
    pub name: String,
    /// Account the configuration is bound to, if any.
    pub account: Option<AccountEmail>,
    /// Active project, if one is set.
    pub project: Option<ProjectId>,
    /// Service account being impersonated, if impersonation is active.
    pub impersonation: Option<ServiceAccount>,
    /// Raw `auth/login_config_file` property (workforce login config path).
    pub login_config_file: Option<String>,
}

impl Context {
    /// Detect the identity kind. Workforce sessions are recognized by the
    /// documented `principal://` prefix on the account; with no account at
    /// all, a configured login config still marks the context as workforce.
    pub fn identity(&self) -> IdentityKind {
        match &self.account {
            Some(account) if account.as_str().starts_with(PRINCIPAL_PREFIX) => {
                IdentityKind::Workforce
            }
            Some(_) => IdentityKind::Google,
            None if self.login_config_file.is_some() => IdentityKind::Workforce,
            None => IdentityKind::Google,
        }
    }
}

/// A GCP project the user can switch to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The immutable project id used in API calls, e.g. `my-project-123`.
    pub id: ProjectId,
    /// Human-readable name shown alongside the id, if the API provided one.
    pub display_name: Option<String>,
}

/// A service account as it appears in the impersonation picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAccountInfo {
    /// The service account's email, used for impersonation.
    pub email: ServiceAccount,
    /// Human-readable name, if one is set.
    pub display_name: Option<String>,
}

/// One gcloud configuration as it appears in listings and pickers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    /// Configuration name (the `<name>` in `configurations/config_<name>`).
    pub name: String,
    /// Account the configuration binds, if any.
    pub account: Option<AccountEmail>,
    /// Project the configuration binds, if any.
    pub project: Option<ProjectId>,
    /// Whether this is the currently active configuration.
    pub is_active: bool,
    /// Raw `auth/login_config_file` property, for workforce re-auth.
    pub login_config_file: Option<String>,
}
