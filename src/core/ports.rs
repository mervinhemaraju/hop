//! Ports: traits describing what core needs from the outside world.
//! Adapters implement them; core never knows which one (rules/architecture.md).

use crate::core::context::Context;
use crate::core::error::ConfigError;

/// Provides the currently active GCP context.
///
/// Implemented by the gcloud config-file adapter in production and by plain
/// fakes in tests.
pub trait ContextSource {
    /// Read the active context.
    fn active_context(&self) -> Result<Context, ConfigError>;
}
