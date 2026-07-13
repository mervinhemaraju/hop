//! Shared credential acquisition with the user's re-auth policy, used by
//! every command that needs an access token (switch, impersonate).

use std::path::Path;

use thiserror::Error;

use crate::core::error::{AuthError, PromptError};
use crate::core::ports::{Authenticator, Confirmer, TokenProvider};
use crate::core::settings::{ReauthPolicy, Settings};
use crate::core::types::{AccessToken, AccountEmail};

/// The auth-related ports a token acquisition needs, plus the workforce
/// login config (if any) so re-auth uses the right flow for the identity.
pub(super) struct AuthFlow<'a> {
    pub tokens: &'a dyn TokenProvider,
    pub authenticator: &'a dyn Authenticator,
    pub confirmer: &'a dyn Confirmer,
    pub login_config: Option<&'a Path>,
}

pub(super) enum TokenOutcome {
    Token(AccessToken),
    /// The user chose not to re-authenticate right now.
    Declined,
}

#[derive(Debug, Error)]
pub(super) enum AuthFlowError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
}

/// Get a token for `account`, applying the reauth policy when the
/// credentials turn out to be expired or revoked.
pub(super) fn token_with_reauth(
    flow: &AuthFlow,
    settings: Settings,
    account: &AccountEmail,
) -> Result<TokenOutcome, AuthFlowError> {
    let token = match flow.tokens.access_token(account) {
        Ok(token) => token,
        Err(err @ AuthError::CredentialsInvalid { .. }) => match settings.reauth {
            ReauthPolicy::Off => return Err(err.into()),
            ReauthPolicy::Auto => {
                flow.authenticator
                    .login(Some(account), false, flow.login_config)?;
                flow.tokens.access_token(account)?
            }
            ReauthPolicy::Prompt => {
                let question = format!("Credentials for {account} are expired. Log in now?");
                match flow.confirmer.confirm(&question) {
                    Ok(Some(true)) => {
                        flow.authenticator
                            .login(Some(account), false, flow.login_config)?;
                        flow.tokens.access_token(account)?
                    }
                    // "No" and Esc both mean: don't log in right now.
                    Ok(Some(false)) | Ok(None) => return Ok(TokenOutcome::Declined),
                    // No terminal to ask on: surface the credential problem
                    // itself, which carries the `hop login` instruction.
                    Err(PromptError::NotInteractive) => return Err(err.into()),
                    Err(other) => return Err(other.into()),
                }
            }
        },
        Err(other) => return Err(other.into()),
    };
    Ok(TokenOutcome::Token(token))
}
