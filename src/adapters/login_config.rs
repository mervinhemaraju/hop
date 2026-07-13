//! Reading workforce login-config files (created by
//! `gcloud iam workforce-pools create-login-config`). Per the docs the file
//! holds endpoints and the audience only, no confidential information, so
//! reading it is safe under rules/gcloud-safety.md.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::core::error::ConfigError;
use crate::core::workforce::WorkforceProvider;

// Doc-confirmed `type` value identifying a workforce login config.
const LOGIN_CONFIG_TYPE: &str = "external_account_authorized_user_login_config";

#[derive(Debug, Deserialize)]
struct LoginConfigFile {
    #[serde(rename = "type")]
    kind: Option<String>,
    audience: Option<String>,
}

/// Load the workforce pool provider a login-config file points at.
pub fn load_workforce_provider(path: &Path) -> Result<WorkforceProvider, ConfigError> {
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Unreadable {
        path: path.to_path_buf(),
        source,
    })?;
    let file: LoginConfigFile =
        serde_json::from_str(&text).map_err(|err| ConfigError::Malformed {
            path: path.to_path_buf(),
            detail: err.to_string(),
        })?;
    if file.kind.as_deref() != Some(LOGIN_CONFIG_TYPE) {
        return Err(ConfigError::Malformed {
            path: path.to_path_buf(),
            detail: format!(
                "not a workforce login config (type {:?}, expected {LOGIN_CONFIG_TYPE:?})",
                file.kind
            ),
        });
    }
    let Some(audience) = file.audience else {
        return Err(ConfigError::Malformed {
            path: path.to_path_buf(),
            detail: "missing audience field".to_string(),
        });
    };
    WorkforceProvider::from_audience(&audience).map_err(|err| ConfigError::Malformed {
        path: path.to_path_buf(),
        detail: err.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_file(test: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("hop-tests")
            .join(format!("{test}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("failed to create scratch dir");
        let path = dir.join("login-config.json");
        fs::write(&path, contents).expect("write failed");
        path
    }

    #[test]
    fn loads_a_documented_login_config() {
        // arrange: fields exactly as create-login-config emits them
        let path = scratch_file(
            "login-config-ok",
            r#"{
                "universe_domain": "",
                "type": "external_account_authorized_user_login_config",
                "audience": "//iam.googleapis.com/locations/global/workforcePools/my-pool/providers/my-okta",
                "auth_url": "https://auth.cloud.google/authorize",
                "token_url": "https://sts.googleapis.com/v1/oauthtoken",
                "token_info_url": "https://sts.googleapis.com/v1/introspect"
            }"#,
        );
        // act
        let provider = load_workforce_provider(&path).expect("load failed");
        // assert
        assert_eq!(provider.pool(), "my-pool");
        assert_eq!(provider.provider(), "my-okta");
    }

    #[test]
    fn rejects_a_wrong_type() {
        // arrange
        let path = scratch_file(
            "login-config-wrong-type",
            r#"{"type": "service_account", "audience": "x"}"#,
        );
        // act
        let err = load_workforce_provider(&path).expect_err("accepted wrong type");
        // assert
        assert!(matches!(err, ConfigError::Malformed { .. }));
    }

    #[test]
    fn rejects_a_missing_audience() {
        // arrange
        let path = scratch_file(
            "login-config-no-audience",
            r#"{"type": "external_account_authorized_user_login_config"}"#,
        );
        // act
        let err = load_workforce_provider(&path).expect_err("accepted missing audience");
        // assert
        assert!(matches!(err, ConfigError::Malformed { .. }));
    }

    #[test]
    fn a_missing_file_is_unreadable() {
        // act
        let err = load_workforce_provider(Path::new("/nonexistent/login-config.json"))
            .expect_err("accepted missing file");
        // assert
        assert!(matches!(err, ConfigError::Unreadable { .. }));
    }
}
