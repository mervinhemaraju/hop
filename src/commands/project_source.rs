//! Shared project listing: turn an account into a pickable list of projects,
//! serving the local cache first and re-authenticating per the user's policy.
//! Both `switch` and `console` use it so the credential-sensitive logic has a
//! single home rather than two copies (rules/security.md, code-style DRY).

use std::path::Path;

use thiserror::Error;

use crate::commands::auth_flow::{AuthFlow, AuthFlowError, TokenOutcome, token_with_reauth};
use crate::core::context::Project;
use crate::core::error::{ApiError, AuthError, ConfigError, PromptError};
use crate::core::ports::{Authenticator, Confirmer, ProjectCache, ProjectLister, TokenProvider};
use crate::core::settings::Settings;
use crate::core::types::AccountEmail;

/// The ports a project listing needs: token acquisition (with re-auth), the
/// remote lister, and the local cache.
pub(super) struct ProjectSourcePorts<'a> {
    pub tokens: &'a dyn TokenProvider,
    pub authenticator: &'a dyn Authenticator,
    pub confirmer: &'a dyn Confirmer,
    pub lister: &'a dyn ProjectLister,
    pub cache: &'a dyn ProjectCache,
}

/// Outcome of obtaining projects.
pub(super) enum Projects {
    List(Vec<Project>),
    /// The user chose not to re-authenticate, so there is nothing to list.
    ReauthDeclined,
}

/// Every way listing projects can fail. Callers map it onto their own error
/// type and exit codes.
#[derive(Debug, Error)]
pub(super) enum ProjectSourceError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error(transparent)]
    Config(#[from] ConfigError),
}

impl From<AuthFlowError> for ProjectSourceError {
    fn from(err: AuthFlowError) -> Self {
        match err {
            AuthFlowError::Auth(err) => Self::Auth(err),
            AuthFlowError::Prompt(err) => Self::Prompt(err),
        }
    }
}

/// Serve projects from the cache when allowed, otherwise fetch via a fresh
/// token, re-authenticating according to the user's policy.
pub(super) fn obtain_projects(
    ports: &ProjectSourcePorts,
    settings: &Settings,
    account: &AccountEmail,
    login_config: Option<&Path>,
    refresh: bool,
) -> Result<Projects, ProjectSourceError> {
    if !refresh {
        if let Some(cached) = ports.cache.cached_projects(account)? {
            if !cached.is_empty() {
                return Ok(Projects::List(cached));
            }
        }
    }
    let flow = AuthFlow {
        tokens: ports.tokens,
        authenticator: ports.authenticator,
        confirmer: ports.confirmer,
        login_config,
    };
    let token = match token_with_reauth(&flow, settings, account)? {
        TokenOutcome::Token(token) => token,
        TokenOutcome::Declined => return Ok(Projects::ReauthDeclined),
    };
    let projects = ports.lister.list_projects(&token)?;
    ports.cache.store_projects(account, &projects)?;
    Ok(Projects::List(projects))
}
