//! Console URL building: pure string work, no browser here.
//!
//! `authuser=<email>` is the account-selection parameter Google documents
//! for Cloud Shell URLs; the console shares the same auth front-end. There
//! is no formal console URL reference, so values are percent-encoded
//! defensively.

use crate::core::types::{AccountEmail, ProjectId};

/// URL of the console dashboard for `project`, pinned to `account` when one
/// is known (so the browser does not open the wrong Google session).
pub fn console_url(project: &ProjectId, account: Option<&AccountEmail>) -> String {
    let mut url = format!(
        "https://console.cloud.google.com/home/dashboard?project={}",
        percent_encode(project.as_str())
    );
    if let Some(account) = account {
        url.push_str("&authuser=");
        url.push_str(&percent_encode(account.as_str()));
    }
    url
}

/// Percent-encode everything but RFC 3986 unreserved characters. std-only;
/// a full URL crate is not warranted for a handful of path and query values.
/// Also used by adapters to keep validated-but-loose identifiers from ever
/// altering a URL path (rules/security.md).
pub fn percent_encode(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_project_url_without_account() {
        // arrange
        let project = ProjectId::new("my-project-123").expect("valid");
        // act
        let url = console_url(&project, None);
        // assert
        assert_eq!(
            url,
            "https://console.cloud.google.com/home/dashboard?project=my-project-123"
        );
    }

    #[test]
    fn pins_the_account_with_percent_encoding() {
        // arrange
        let project = ProjectId::new("my-project-123").expect("valid");
        let account = AccountEmail::new("dev@example.com").expect("valid");
        // act
        let url = console_url(&project, Some(&account));
        // assert
        assert_eq!(
            url,
            "https://console.cloud.google.com/home/dashboard?project=my-project-123&authuser=dev%40example.com"
        );
    }
}
