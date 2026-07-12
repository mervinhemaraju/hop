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
