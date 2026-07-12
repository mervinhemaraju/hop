use std::process::ExitCode;

use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::core::error::ConfigError;
use crate::core::ports::ContextSource;

/// Show the active gcloud context: configuration, account, project, and
/// impersonation state.
pub fn run() -> ExitCode {
    // Composition root: the production adapter is chosen here and only here;
    // everything below sees just the port.
    let source = match GcloudConfigSource::new() {
        Ok(source) => source,
        Err(err) => {
            eprintln!("hop status: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("config directory:     {}", source.config_dir().display());
    match run_with(&source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hop status: {err}");
            ExitCode::FAILURE
        }
    }
}

// The testable body of the command: works against any ContextSource. Returns
// Result rather than ExitCode because ExitCode is opaque (no PartialEq);
// run() owns the mapping to process exit status.
fn run_with(source: &impl ContextSource) -> Result<(), ConfigError> {
    let context = source.active_context()?;
    eprintln!("active configuration: {}", context.name);
    eprintln!(
        "account:              {}",
        display_or_unset(context.account.as_ref())
    );
    eprintln!(
        "project:              {}",
        display_or_unset(context.project.as_ref())
    );
    eprintln!(
        "impersonation:        {}",
        display_or_unset(context.impersonation.as_ref())
    );
    Ok(())
}

fn display_or_unset(value: Option<&impl std::fmt::Display>) -> String {
    value.map_or_else(|| "(not set)".to_string(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::Context;

    /// Plain fake: hands out a fixed context, no filesystem involved.
    struct FakeSource(Context);

    impl ContextSource for FakeSource {
        fn active_context(&self) -> Result<Context, ConfigError> {
            Ok(self.0.clone())
        }
    }

    /// Fake that always fails. The error is constructed fresh per call
    /// because ConfigError holds io::Error variants and is not Clone.
    struct FailingSource;

    impl ContextSource for FailingSource {
        fn active_context(&self) -> Result<Context, ConfigError> {
            Err(ConfigError::HomeDirUnavailable)
        }
    }

    #[test]
    fn run_with_succeeds_when_the_source_provides_a_context() {
        // arrange
        let source = FakeSource(Context {
            name: "work".to_string(),
            account: None,
            project: None,
            impersonation: None,
        });
        // act
        let result = run_with(&source);
        // assert
        assert!(result.is_ok());
    }

    #[test]
    fn run_with_propagates_the_source_error() {
        // arrange
        let source = FailingSource;
        // act
        let result = run_with(&source);
        // assert
        assert!(matches!(result, Err(ConfigError::HomeDirUnavailable)));
    }
}
