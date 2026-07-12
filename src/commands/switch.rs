use std::process::ExitCode;

use thiserror::Error;

use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::prompt::InquirePicker;
use crate::commands::{EXIT_BAD_INPUT, EXIT_CANCELLED, EXIT_NOT_INTERACTIVE};
use crate::core::error::{ConfigError, PromptError};
use crate::core::ports::{ConfigurationPicker, ConfigurationStore};

// Either half of the switch flow can fail; this keeps `?` working across
// both while the exit-code mapping stays in one place.
#[derive(Debug, Error)]
enum SwitchError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
}

impl SwitchError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Config(ConfigError::UnknownConfiguration { .. }) => {
                ExitCode::from(EXIT_BAD_INPUT)
            }
            Self::Prompt(PromptError::NotInteractive) => ExitCode::from(EXIT_NOT_INTERACTIVE),
            _ => ExitCode::FAILURE,
        }
    }
}

#[derive(Debug)]
enum Outcome {
    Switched(String),
    AlreadyActive(String),
}

/// Switch the active gcloud configuration, interactively or by name.
pub fn run(name: Option<&str>) -> ExitCode {
    // Composition root: production adapters are chosen here and only here.
    let store = match GcloudConfigSource::new() {
        Ok(store) => store,
        Err(err) => {
            eprintln!("hop switch: {err}");
            return ExitCode::FAILURE;
        }
    };
    match select_and_activate(&store, &InquirePicker, name) {
        Ok(Some(Outcome::Switched(name))) => {
            eprintln!("switched to {name}");
            ExitCode::SUCCESS
        }
        Ok(Some(Outcome::AlreadyActive(name))) => {
            eprintln!("already on {name}");
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!("cancelled");
            ExitCode::from(EXIT_CANCELLED)
        }
        Err(err) => {
            eprintln!("hop switch: {err}");
            err.exit_code()
        }
    }
}

// The testable body: resolve the target (argument or picker), then activate.
// Ok(None) means the user cancelled the picker.
fn select_and_activate(
    store: &impl ConfigurationStore,
    picker: &impl ConfigurationPicker,
    name: Option<&str>,
) -> Result<Option<Outcome>, SwitchError> {
    let configurations = store.list()?;
    if configurations.is_empty() {
        return Err(ConfigError::NoConfigurations.into());
    }
    let target = match name {
        Some(name) => name.to_string(),
        None => match picker.pick(&configurations)? {
            Some(choice) => choice,
            None => return Ok(None),
        },
    };
    if configurations
        .iter()
        .any(|c| c.name == target && c.is_active)
    {
        return Ok(Some(Outcome::AlreadyActive(target)));
    }
    store.activate(&target)?;
    Ok(Some(Outcome::Switched(target)))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::core::context::Configuration;

    /// In-memory store; RefCell gives the &self trait method a way to
    /// record the activation for assertions.
    struct FakeStore {
        configurations: Vec<Configuration>,
        activated: RefCell<Option<String>>,
    }

    impl FakeStore {
        fn with(names_active: &[(&str, bool)]) -> Self {
            Self {
                configurations: names_active
                    .iter()
                    .map(|(name, is_active)| Configuration {
                        name: name.to_string(),
                        account: None,
                        project: None,
                        is_active: *is_active,
                    })
                    .collect(),
                activated: RefCell::new(None),
            }
        }
    }

    impl ConfigurationStore for FakeStore {
        fn list(&self) -> Result<Vec<Configuration>, ConfigError> {
            Ok(self.configurations.clone())
        }

        fn activate(&self, name: &str) -> Result<(), ConfigError> {
            if !self.configurations.iter().any(|c| c.name == name) {
                return Err(ConfigError::UnknownConfiguration {
                    name: name.to_string(),
                });
            }
            *self.activated.borrow_mut() = Some(name.to_string());
            Ok(())
        }
    }

    struct FakePicker(Option<String>);

    impl ConfigurationPicker for FakePicker {
        fn pick(&self, _: &[Configuration]) -> Result<Option<String>, PromptError> {
            Ok(self.0.clone())
        }
    }

    struct UnusablePicker;

    impl ConfigurationPicker for UnusablePicker {
        fn pick(&self, _: &[Configuration]) -> Result<Option<String>, PromptError> {
            panic!("picker must not run when a name is given");
        }
    }

    #[test]
    fn switches_by_name_without_the_picker() {
        // arrange
        let store = FakeStore::with(&[("default", true), ("work", false)]);
        // act
        let outcome =
            select_and_activate(&store, &UnusablePicker, Some("work")).expect("switch failed");
        // assert
        assert!(matches!(outcome, Some(Outcome::Switched(name)) if name == "work"));
        assert_eq!(store.activated.borrow().as_deref(), Some("work"));
    }

    #[test]
    fn reports_already_active_without_writing() {
        // arrange
        let store = FakeStore::with(&[("default", true)]);
        // act
        let outcome =
            select_and_activate(&store, &UnusablePicker, Some("default")).expect("switch failed");
        // assert
        assert!(matches!(outcome, Some(Outcome::AlreadyActive(name)) if name == "default"));
        assert_eq!(store.activated.borrow().as_deref(), None);
    }

    #[test]
    fn switches_via_the_picker() {
        // arrange
        let store = FakeStore::with(&[("default", true), ("work", false)]);
        let picker = FakePicker(Some("work".to_string()));
        // act
        let outcome = select_and_activate(&store, &picker, None).expect("switch failed");
        // assert
        assert!(matches!(outcome, Some(Outcome::Switched(name)) if name == "work"));
        assert_eq!(store.activated.borrow().as_deref(), Some("work"));
    }

    #[test]
    fn a_cancelled_picker_is_not_an_error() {
        // arrange
        let store = FakeStore::with(&[("default", true)]);
        let picker = FakePicker(None);
        // act
        let outcome = select_and_activate(&store, &picker, None).expect("cancel became error");
        // assert
        assert!(outcome.is_none());
        assert_eq!(store.activated.borrow().as_deref(), None);
    }

    #[test]
    fn an_unknown_name_maps_to_bad_input() {
        // arrange
        let store = FakeStore::with(&[("default", true)]);
        // act
        let err = select_and_activate(&store, &UnusablePicker, Some("nope"))
            .expect_err("activated a ghost");
        // assert
        assert!(matches!(
            err,
            SwitchError::Config(ConfigError::UnknownConfiguration { .. })
        ));
    }

    #[test]
    fn no_configurations_at_all_is_an_error() {
        // arrange
        let store = FakeStore::with(&[]);
        // act
        let err =
            select_and_activate(&store, &FakePicker(None), None).expect_err("empty store accepted");
        // assert
        assert!(matches!(
            err,
            SwitchError::Config(ConfigError::NoConfigurations)
        ));
    }
}
