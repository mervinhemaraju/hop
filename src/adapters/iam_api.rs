//! IAM and IAM Credentials API clients for impersonation.
//! Endpoints and fields verified against the official docs on 2026-07-12:
//! `GET /v1/projects/{project}/serviceAccounts` (pageSize max 100) and
//! `POST /v1/projects/-/serviceAccounts/{email}:generateAccessToken`.

use std::time::Duration;

use serde::Deserialize;

use crate::core::console::percent_encode;
use crate::core::context::ServiceAccountInfo;
use crate::core::error::ApiError;
use crate::core::ports::{ImpersonationVerifier, ServiceAccountLister};
use crate::core::types::{AccessToken, ProjectId, ServiceAccount};

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(default)]
    accounts: Vec<ServiceAccountDto>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountDto {
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    disabled: bool,
}

/// Direct HTTPS access to the IAM APIs.
pub struct IamApi {
    agent: ureq::Agent,
}

impl IamApi {
    /// Client with a global timeout so a dead network can't hang a prompt.
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30)))
                .build()
                .new_agent(),
        }
    }
}

impl ServiceAccountLister for IamApi {
    fn list_service_accounts(
        &self,
        token: &AccessToken,
        project: &ProjectId,
    ) -> Result<Vec<ServiceAccountInfo>, ApiError> {
        // Identifiers are validated but loosely; encoding keeps them from
        // ever altering the URL path (rules/security.md).
        let url = format!(
            "https://iam.googleapis.com/v1/projects/{}/serviceAccounts",
            percent_encode(project.as_str())
        );
        let mut accounts = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut request = self
                .agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", token.secret()))
                // Documented maximum; keeps round-trips down.
                .query("pageSize", "100");
            if let Some(page) = &page_token {
                request = request.query("pageToken", page);
            }
            let mut response = request.call().map_err(map_transport_error)?;
            let body: ListResponse = response
                .body_mut()
                .read_json()
                .map_err(|err| ApiError::Decode(err.to_string()))?;
            for dto in body.accounts {
                if let Some(info) = to_info(dto)? {
                    accounts.push(info);
                }
            }
            match body.next_page_token {
                Some(next) if !next.is_empty() => page_token = Some(next),
                _ => break,
            }
        }
        accounts.sort_by(|a, b| a.email.as_str().cmp(b.email.as_str()));
        Ok(accounts)
    }
}

impl ImpersonationVerifier for IamApi {
    fn verify_impersonation(
        &self,
        token: &AccessToken,
        service_account: &ServiceAccount,
    ) -> Result<(), ApiError> {
        // The email is encoded; the literal `:generateAccessToken` verb
        // suffix must stay unencoded.
        let url = format!(
            "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{}:generateAccessToken",
            percent_encode(service_account.as_str())
        );
        // A minimal, short-lived mint proves the permission. The response
        // body carries the minted token, so it is deliberately never read:
        // a 200 status is all the proof needed (rules/security.md).
        self.agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", token.secret()))
            .send_json(serde_json::json!({
                "scope": ["https://www.googleapis.com/auth/cloud-platform"],
                "lifetime": "300s",
            }))
            .map_err(map_transport_error)?;
        Ok(())
    }
}

// Map one wire account into the domain, dropping disabled ones.
fn to_info(dto: ServiceAccountDto) -> Result<Option<ServiceAccountInfo>, ApiError> {
    if dto.disabled {
        return Ok(None);
    }
    let Some(raw_email) = dto.email else {
        return Ok(None);
    };
    let email = ServiceAccount::new(raw_email).map_err(|err| ApiError::Decode(err.to_string()))?;
    Ok(Some(ServiceAccountInfo {
        email,
        display_name: dto.display_name.filter(|name| !name.is_empty()),
    }))
}

fn map_transport_error(err: ureq::Error) -> ApiError {
    match err {
        ureq::Error::StatusCode(code) => ApiError::Status(code),
        other => ApiError::Network(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_documented_list_page_and_drops_disabled() {
        // arrange: field names exactly as documented for serviceAccounts.list
        let json = r#"{
            "accounts": [
                {"email": "deploy@my-project-123.iam.gserviceaccount.com", "displayName": "Deploy", "disabled": false},
                {"email": "old@my-project-123.iam.gserviceaccount.com", "disabled": true},
                {"email": "bare@my-project-123.iam.gserviceaccount.com"}
            ],
            "nextPageToken": "tok456"
        }"#;
        // act
        let page: ListResponse = serde_json::from_str(json).expect("decode failed");
        let accounts: Vec<ServiceAccountInfo> = page
            .accounts
            .into_iter()
            .filter_map(|dto| to_info(dto).expect("mapping failed"))
            .collect();
        // assert
        assert_eq!(page.next_page_token.as_deref(), Some("tok456"));
        assert_eq!(accounts.len(), 2);
        assert_eq!(
            accounts[0].email.as_str(),
            "deploy@my-project-123.iam.gserviceaccount.com"
        );
        assert_eq!(accounts[0].display_name.as_deref(), Some("Deploy"));
        assert_eq!(accounts[1].display_name, None);
    }

    #[test]
    fn tolerates_an_empty_list_response() {
        // act
        let page: ListResponse = serde_json::from_str("{}").expect("decode failed");
        // assert
        assert!(page.accounts.is_empty());
        assert_eq!(page.next_page_token, None);
    }
}
