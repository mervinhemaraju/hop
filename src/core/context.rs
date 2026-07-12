//! The active GCP context as hop understands it: which gcloud
//! configuration is active and what it binds.

use crate::core::types::{AccountEmail, ProjectId, ServiceAccount};

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
}

/// A GCP project the user can switch to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The immutable project id used in API calls, e.g. `my-project-123`.
    pub id: ProjectId,
    /// Human-readable name shown alongside the id, if the API provided one.
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
}
