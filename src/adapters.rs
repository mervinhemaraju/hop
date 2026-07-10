//! Adapter layer: everything that touches the outside world (filesystem,
//! gcloud state, network, browser). All platform-specific code lives here
//! (rules/cross-platform.md).

pub mod gcloud_config;
