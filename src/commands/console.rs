use std::path::Path;
use std::process::ExitCode;

use crate::adapters::browser::SystemBrowser;
use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::login_config::load_workforce_provider;
use crate::commands::EXIT_BAD_INPUT;
use crate::core::console::{console_url, federated_console_url};
use crate::core::context::IdentityKind;
use crate::core::ports::{BrowserOpener, ContextSource};
use crate::core::types::ProjectId;

/// Open the GCP console in the browser for the active (or given) context.
pub fn run(project: Option<&str>, url_only: bool) -> ExitCode {
    // Composition root; the URL building itself is pure core logic.
    let source = match GcloudConfigSource::new() {
        Ok(source) => source,
        Err(err) => return fail(&err.to_string()),
    };
    let context = match source.active_context() {
        Ok(context) => context,
        Err(err) => return fail(&err.to_string()),
    };
    let project = match project {
        Some(raw) => match ProjectId::new(raw) {
            Ok(project) => project,
            Err(err) => {
                eprintln!("hop console: invalid project id: {err}");
                return ExitCode::from(EXIT_BAD_INPUT);
            }
        },
        None => match context.project.clone() {
            Some(project) => project,
            None => {
                return fail(
                    "no project in the active configuration; pass --project <id> or set one with `hop switch`",
                );
            }
        },
    };
    // Workforce sessions go through the federated console sign-in URL; the
    // standard console URL would prompt for a Google account they lack.
    let url = match context.identity() {
        IdentityKind::Workforce => {
            let Some(raw_path) = context.login_config_file.as_deref() else {
                return fail(
                    "workforce session, but the configuration has no auth/login_config_file property; re-run `gcloud iam workforce-pools create-login-config <provider> --activate`",
                );
            };
            match load_workforce_provider(Path::new(raw_path)) {
                Ok(provider) => federated_console_url(&provider, Some(&project)),
                Err(err) => return fail(&err.to_string()),
            }
        }
        IdentityKind::Google => console_url(&project, context.account.as_ref()),
    };
    if url_only {
        // stdout on purpose: this is machine-consumable output
        // (rules/cli-ux.md), e.g. `open "$(hop console --url)"`.
        println!("{url}");
        return ExitCode::SUCCESS;
    }
    eprintln!("opening {url}");
    match SystemBrowser.open_url(&url) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&err.to_string()),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hop console: {message}");
    ExitCode::FAILURE
}
