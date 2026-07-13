//! Adapter layer: everything that touches the outside world (filesystem,
//! gcloud state, network, browser). All platform-specific code lives here
//! (rules/cross-platform.md).

pub mod browser;
pub mod gcloud_config;
pub mod gcloud_ini;
pub mod gcloud_process;
pub mod hop_files;
pub mod iam_api;
pub mod login_config;
pub mod prompt;
pub mod resource_manager;
