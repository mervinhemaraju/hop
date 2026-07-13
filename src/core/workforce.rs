//! Workforce identity federation domain logic: pure parsing of the
//! identifiers involved, no file or network access.
//!
//! Doc-confirmed shapes (2026-07-12):
//! - login-config `audience`:
//!   `//iam.googleapis.com/locations/global/workforcePools/<pool>/providers/<provider>`
//! - principal identifier:
//!   `principal://iam.googleapis.com/locations/global/workforcePools/<pool>/subject/<subject>`

use thiserror::Error;

/// The documented prefix of workforce principal identifiers; the signal by
/// which hop recognizes a workforce session in gcloud state.
pub const PRINCIPAL_PREFIX: &str = "principal://";

const AUDIENCE_PREFIX: &str = "//iam.googleapis.com/locations/global/workforcePools/";

/// Failures interpreting workforce identifiers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkforceError {
    #[error(
        "unexpected login-config audience {audience:?}; expected //iam.googleapis.com/locations/global/workforcePools/<pool>/providers/<provider>"
    )]
    MalformedAudience { audience: String },
}

/// A workforce pool provider, as extracted from a login-config audience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkforceProvider {
    pool: String,
    provider: String,
}

impl WorkforceProvider {
    /// Parse the `audience` field of a workforce login-config file.
    pub fn from_audience(audience: &str) -> Result<Self, WorkforceError> {
        let malformed = || WorkforceError::MalformedAudience {
            audience: audience.to_string(),
        };
        let rest = audience
            .strip_prefix(AUDIENCE_PREFIX)
            .ok_or_else(malformed)?;
        let (pool, provider) = rest.split_once("/providers/").ok_or_else(malformed)?;
        if pool.is_empty() || provider.is_empty() || pool.contains('/') || provider.contains('/') {
            return Err(malformed());
        }
        Ok(Self {
            pool: pool.to_string(),
            provider: provider.to_string(),
        })
    }

    /// The workforce pool id.
    pub fn pool(&self) -> &str {
        &self.pool
    }

    /// The provider id within the pool.
    pub fn provider(&self) -> &str {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_documented_audience() {
        // arrange
        let audience =
            "//iam.googleapis.com/locations/global/workforcePools/my-pool/providers/my-okta";
        // act
        let provider = WorkforceProvider::from_audience(audience).expect("parse failed");
        // assert
        assert_eq!(provider.pool(), "my-pool");
        assert_eq!(provider.provider(), "my-okta");
    }

    #[test]
    fn rejects_a_wrong_prefix() {
        // act
        let err = WorkforceProvider::from_audience("//iam.googleapis.com/projects/123")
            .expect_err("accepted");
        // assert
        assert!(matches!(err, WorkforceError::MalformedAudience { .. }));
    }

    #[test]
    fn rejects_a_missing_providers_segment() {
        // act
        let err = WorkforceProvider::from_audience(
            "//iam.googleapis.com/locations/global/workforcePools/my-pool",
        )
        .expect_err("accepted");
        // assert
        assert!(matches!(err, WorkforceError::MalformedAudience { .. }));
    }

    #[test]
    fn rejects_empty_or_nested_segments() {
        // arrange
        let cases = [
            "//iam.googleapis.com/locations/global/workforcePools//providers/x",
            "//iam.googleapis.com/locations/global/workforcePools/p/providers/",
            "//iam.googleapis.com/locations/global/workforcePools/p/extra/providers/x",
        ];
        for audience in cases {
            // act + assert
            assert!(
                WorkforceProvider::from_audience(audience).is_err(),
                "accepted: {audience}"
            );
        }
    }
}
