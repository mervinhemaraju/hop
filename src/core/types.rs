//! Validated newtypes for GCP identifiers, so a project id can never be
//! passed where an account email is expected (rules/architecture.md).

use std::fmt;

use crate::core::error::ValidationError;

// Shared by all identifier newtypes. Deliberately loose: exact GCP charset
// rules are confirmed via /gcp-check when a phase needs them; this only
// guarantees a non-empty, single-token, visible-ASCII value that is safe to
// pass as a process argument (rules/security.md).
fn validate(raw: &str, what: &'static str) -> Result<(), ValidationError> {
    if raw.is_empty() {
        return Err(ValidationError::Empty { what });
    }
    // is_ascii_graphic covers printable ASCII except space, so this rejects
    // whitespace, control characters, and non-ASCII in one pass.
    if let Some(found) = raw.chars().find(|c| !c.is_ascii_graphic()) {
        return Err(ValidationError::InvalidCharacter { what, found });
    }
    Ok(())
}

/// A GCP project id, e.g. `my-project-123`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectId(String);

impl ProjectId {
    /// Validate and wrap a raw project id.
    pub fn new(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = raw.into();
        validate(&raw, "project id")?;
        Ok(Self(raw))
    }

    /// The id as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An authenticated gcloud account email, e.g. `dev@example.com`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountEmail(String);

impl AccountEmail {
    /// Validate and wrap a raw account email.
    pub fn new(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = raw.into();
        validate(&raw, "account email")?;
        Ok(Self(raw))
    }

    /// The email as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A service account to impersonate,
/// e.g. `sa@my-project-123.iam.gserviceaccount.com`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAccount(String);

impl ServiceAccount {
    /// Validate and wrap a raw service account email.
    pub fn new(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = raw.into();
        validate(&raw, "service account")?;
        Ok(Self(raw))
    }

    /// The service account email as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A short-lived OAuth2 access token.
///
/// Deliberately has no `Display` and a redacted `Debug` so it cannot leak
/// into logs or error messages (rules/security.md). The accessor is named
/// `secret` so every exposure site is easy to audit.
#[derive(Clone)]
pub struct AccessToken(String);

impl AccessToken {
    /// Wrap a raw token, trimming the trailing newline gcloud prints.
    pub fn new(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::Empty {
                what: "access token",
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// The raw token value, for Authorization headers only.
    pub fn secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_id_accepts_a_valid_value() {
        // arrange
        let raw = "my-project-123";
        // act
        let id = ProjectId::new(raw).expect("valid project id was rejected");
        // assert
        assert_eq!(id.as_str(), raw);
        assert_eq!(id.to_string(), raw);
    }

    #[test]
    fn project_id_accepts_an_owned_string() {
        // arrange
        let raw = String::from("my-project-123");
        // act
        let id = ProjectId::new(raw).expect("valid project id was rejected");
        // assert
        assert_eq!(id.as_str(), "my-project-123");
    }

    #[test]
    fn project_id_rejects_empty_input() {
        // act
        let err = ProjectId::new("").expect_err("empty project id was accepted");
        // assert
        assert_eq!(err, ValidationError::Empty { what: "project id" });
    }

    #[test]
    fn project_id_rejects_whitespace() {
        // act
        let err = ProjectId::new("my project").expect_err("project id with a space was accepted");
        // assert
        assert_eq!(
            err,
            ValidationError::InvalidCharacter {
                what: "project id",
                found: ' ',
            }
        );
    }

    #[test]
    fn account_email_accepts_a_valid_value() {
        // arrange
        let raw = "dev@example.com";
        // act
        let email = AccountEmail::new(raw).expect("valid account email was rejected");
        // assert
        assert_eq!(email.as_str(), raw);
        assert_eq!(email.to_string(), raw);
    }

    #[test]
    fn account_email_rejects_control_characters() {
        // act
        let err = AccountEmail::new("dev@example.com\n")
            .expect_err("account email with a newline was accepted");
        // assert
        assert_eq!(
            err,
            ValidationError::InvalidCharacter {
                what: "account email",
                found: '\n',
            }
        );
    }

    #[test]
    fn service_account_accepts_a_valid_value() {
        // arrange
        let raw = "sa@my-project-123.iam.gserviceaccount.com";
        // act
        let sa = ServiceAccount::new(raw).expect("valid service account was rejected");
        // assert
        assert_eq!(sa.as_str(), raw);
        assert_eq!(sa.to_string(), raw);
    }

    #[test]
    fn service_account_rejects_non_ascii() {
        // act
        let err = ServiceAccount::new("sa@münchen.example")
            .expect_err("service account with non-ASCII was accepted");
        // assert
        assert_eq!(
            err,
            ValidationError::InvalidCharacter {
                what: "service account",
                found: 'ü',
            }
        );
    }
}
