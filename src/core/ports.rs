//! Ports: traits describing what core needs from the outside world.
//! Adapters implement them; core never knows which one (rules/architecture.md).

use crate::core::context::{Configuration, Context};
use crate::core::error::{ConfigError, PromptError};

/// Provides the currently active GCP context.
///
/// Implemented by the gcloud config-file adapter in production and by plain
/// fakes in tests.
pub trait ContextSource {
    /// Read the active context.
    fn active_context(&self) -> Result<Context, ConfigError>;
}

/// Lists gcloud configurations and switches the active one.
pub trait ConfigurationStore {
    /// All configurations, sorted by name.
    fn list(&self) -> Result<Vec<Configuration>, ConfigError>;
    /// Make `name` the active configuration; fails if it does not exist.
    fn activate(&self, name: &str) -> Result<(), ConfigError>;
}

/// Lets the user choose a configuration interactively.
pub trait ConfigurationPicker {
    /// Present the choices; `Ok(None)` means the user cancelled.
    fn pick(&self, configurations: &[Configuration]) -> Result<Option<String>, PromptError>;
}
