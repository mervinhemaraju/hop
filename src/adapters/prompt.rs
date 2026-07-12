//! Interactive prompts, rendered on stderr so stdout stays machine-clean
//! (rules/cli-ux.md). No unit tests here: the adapter is a thin mapping
//! around inquire and needs a real TTY; command logic is tested via fakes.

use std::env;
use std::fmt;
use std::io::{IsTerminal, stderr, stdin};

use inquire::ui::RenderConfig;
use inquire::{InquireError, Select};

use crate::core::context::Configuration;
use crate::core::error::PromptError;
use crate::core::ports::ConfigurationPicker;

/// Arrow-key picker with fuzzy filtering, backed by inquire.
pub struct InquirePicker;

// Select needs Display items; carry the name alongside the rendered label.
struct Choice {
    name: String,
    label: String,
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

impl ConfigurationPicker for InquirePicker {
    fn pick(&self, configurations: &[Configuration]) -> Result<Option<String>, PromptError> {
        // Never block on a prompt without a terminal (rules/cli-ux.md).
        if !stdin().is_terminal() || !stderr().is_terminal() {
            return Err(PromptError::NotInteractive);
        }
        let width = configurations
            .iter()
            .map(|c| c.name.len())
            .max()
            .unwrap_or(0);
        let choices: Vec<Choice> = configurations
            .iter()
            .map(|c| Choice {
                name: c.name.clone(),
                label: label(c, width),
            })
            .collect();
        let selection = Select::new("Switch to configuration:", choices)
            .with_render_config(render_config())
            .prompt();
        match selection {
            Ok(choice) => Ok(Some(choice.name)),
            // Esc and Ctrl+C are a decision, not a failure (rules/cli-ux.md).
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
            Err(err) => Err(PromptError::Backend(err.to_string())),
        }
    }
}

fn label(configuration: &Configuration, width: usize) -> String {
    let account = configuration
        .account
        .as_ref()
        .map_or_else(|| "(no account)".to_string(), ToString::to_string);
    let mut label = format!("{:<width$}  {account}", configuration.name);
    if let Some(project) = &configuration.project {
        label.push_str(&format!(" / {project}"));
    }
    if configuration.is_active {
        label.push_str("  (active)");
    }
    label
}

// Honor NO_COLOR (https://no-color.org): set and non-empty disables color.
fn render_config() -> RenderConfig<'static> {
    match env::var_os("NO_COLOR") {
        Some(value) if !value.is_empty() => RenderConfig::empty(),
        _ => RenderConfig::default_colored(),
    }
}
