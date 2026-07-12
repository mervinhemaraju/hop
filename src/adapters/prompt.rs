//! Interactive prompts, rendered on stderr so stdout stays machine-clean
//! (rules/cli-ux.md). No unit tests here: the adapter is a thin mapping
//! around inquire and needs a real TTY; command logic is tested via fakes.

use std::env;
use std::fmt;
use std::io::{IsTerminal, stderr, stdin};

use inquire::ui::RenderConfig;
use inquire::{Confirm, InquireError, Select};

use crate::core::context::{Configuration, Project};
use crate::core::error::PromptError;
use crate::core::ports::{ConfigurationPicker, Confirmer, ProjectPicker};
use crate::core::types::ProjectId;

/// Arrow-key prompts with fuzzy filtering, backed by inquire.
pub struct InquirePicker;

// Select needs Display items; carry the picked value alongside its label.
struct Choice<T> {
    value: T,
    label: String,
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

impl ConfigurationPicker for InquirePicker {
    fn pick(&self, configurations: &[Configuration]) -> Result<Option<String>, PromptError> {
        require_terminal()?;
        let width = configurations
            .iter()
            .map(|c| c.name.len())
            .max()
            .unwrap_or(0);
        let choices: Vec<Choice<String>> = configurations
            .iter()
            .map(|c| Choice {
                value: c.name.clone(),
                label: configuration_label(c, width),
            })
            .collect();
        select("Switch to configuration:", choices)
    }
}

impl ProjectPicker for InquirePicker {
    fn available(&self) -> bool {
        require_terminal().is_ok()
    }

    fn pick(&self, projects: &[Project]) -> Result<Option<ProjectId>, PromptError> {
        require_terminal()?;
        let width = projects
            .iter()
            .map(|p| p.id.as_str().len())
            .max()
            .unwrap_or(0);
        let choices: Vec<Choice<ProjectId>> = projects
            .iter()
            .map(|p| Choice {
                value: p.id.clone(),
                label: project_label(p, width),
            })
            .collect();
        select("Switch to project:", choices)
    }
}

impl Confirmer for InquirePicker {
    fn confirm(&self, message: &str) -> Result<Option<bool>, PromptError> {
        require_terminal()?;
        let answer = Confirm::new(message)
            .with_default(true)
            .with_render_config(render_config())
            .prompt();
        match answer {
            Ok(choice) => Ok(Some(choice)),
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
            Err(err) => Err(PromptError::Backend(err.to_string())),
        }
    }
}

// Never block on a prompt without a terminal (rules/cli-ux.md).
fn require_terminal() -> Result<(), PromptError> {
    if stdin().is_terminal() && stderr().is_terminal() {
        Ok(())
    } else {
        Err(PromptError::NotInteractive)
    }
}

// Shared Select runner: Esc and Ctrl+C are a decision, not a failure
// (rules/cli-ux.md), so they map to Ok(None).
fn select<T>(message: &str, choices: Vec<Choice<T>>) -> Result<Option<T>, PromptError> {
    let selection = Select::new(message, choices)
        .with_render_config(render_config())
        .prompt();
    match selection {
        Ok(choice) => Ok(Some(choice.value)),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(err) => Err(PromptError::Backend(err.to_string())),
    }
}

fn configuration_label(configuration: &Configuration, width: usize) -> String {
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

fn project_label(project: &Project, width: usize) -> String {
    match &project.display_name {
        // Pad the &str, not the newtype: width formatting only applies to
        // types whose Display uses f.pad, and ours writes straight through.
        Some(name) => format!("{:<width$}  {name}", project.id.as_str()),
        None => project.id.to_string(),
    }
}

// Honor NO_COLOR (https://no-color.org): set and non-empty disables color.
fn render_config() -> RenderConfig<'static> {
    match env::var_os("NO_COLOR") {
        Some(value) if !value.is_empty() => RenderConfig::empty(),
        _ => RenderConfig::default_colored(),
    }
}
