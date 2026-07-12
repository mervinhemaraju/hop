//! Cloud Resource Manager v3 client for project listing.
//! Endpoint and field names verified against the official docs on
//! 2026-07-12 (`GET /v3/projects:search`, `projects[]`, `nextPageToken`).

use std::time::Duration;

use serde::Deserialize;

use crate::core::context::Project;
use crate::core::error::ApiError;
use crate::core::ports::ProjectLister;
use crate::core::types::{AccessToken, ProjectId};

const SEARCH_URL: &str = "https://cloudresourcemanager.googleapis.com/v3/projects:search";

/// Wire format of one page of `projects:search`.
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    projects: Vec<ProjectDto>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectDto {
    #[serde(rename = "projectId")]
    project_id: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    state: Option<String>,
}

/// Direct HTTPS access to the Resource Manager API.
pub struct ResourceManagerApi {
    agent: ureq::Agent,
}

impl ResourceManagerApi {
    /// Client with a global timeout so a dead network can't hang the picker.
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30)))
                .build()
                .new_agent(),
        }
    }
}

impl ProjectLister for ResourceManagerApi {
    fn list_projects(&self, token: &AccessToken) -> Result<Vec<Project>, ApiError> {
        let mut projects = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut request = self
                .agent
                .get(SEARCH_URL)
                .header("Authorization", &format!("Bearer {}", token.secret()))
                // The server clamps oversized pages; asking big keeps the
                // number of round-trips down.
                .query("pageSize", "500");
            if let Some(page) = &page_token {
                request = request.query("pageToken", page);
            }
            let mut response = request.call().map_err(map_transport_error)?;
            let body: SearchResponse = response
                .body_mut()
                .read_json()
                .map_err(|err| ApiError::Decode(err.to_string()))?;
            for dto in body.projects {
                if let Some(project) = to_project(dto)? {
                    projects.push(project);
                }
            }
            match body.next_page_token {
                Some(next) if !next.is_empty() => page_token = Some(next),
                _ => break,
            }
        }
        projects.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(projects)
    }
}

// Map one wire project into the domain, dropping non-ACTIVE ones (the API
// also returns projects pending deletion).
fn to_project(dto: ProjectDto) -> Result<Option<Project>, ApiError> {
    if dto.state.as_deref() != Some("ACTIVE") {
        return Ok(None);
    }
    let Some(raw_id) = dto.project_id else {
        return Ok(None);
    };
    let id = ProjectId::new(raw_id).map_err(|err| ApiError::Decode(err.to_string()))?;
    Ok(Some(Project {
        id,
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
    fn decodes_a_documented_response_page() {
        // arrange: field names exactly as documented for v3 projects:search
        let json = r#"{
            "projects": [
                {"projectId": "my-project-123", "displayName": "My Project", "state": "ACTIVE"},
                {"projectId": "doomed-project", "displayName": "Old", "state": "DELETE_REQUESTED"},
                {"projectId": "bare-project", "state": "ACTIVE"}
            ],
            "nextPageToken": "tok123"
        }"#;
        // act
        let page: SearchResponse = serde_json::from_str(json).expect("decode failed");
        let projects: Vec<Project> = page
            .projects
            .into_iter()
            .filter_map(|dto| to_project(dto).expect("mapping failed"))
            .collect();
        // assert: the DELETE_REQUESTED project is dropped
        assert_eq!(page.next_page_token.as_deref(), Some("tok123"));
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].id.as_str(), "my-project-123");
        assert_eq!(projects[0].display_name.as_deref(), Some("My Project"));
        assert_eq!(projects[1].id.as_str(), "bare-project");
        assert_eq!(projects[1].display_name, None);
    }

    #[test]
    fn tolerates_an_empty_response() {
        // act: a response with no projects field at all
        let page: SearchResponse = serde_json::from_str("{}").expect("decode failed");
        // assert
        assert!(page.projects.is_empty());
        assert_eq!(page.next_page_token, None);
    }
}
