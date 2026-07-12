use std::process::ExitCode;

use crate::adapters::browser::SystemBrowser;
use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::commands::EXIT_BAD_INPUT;
use crate::core::console::console_url;
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
    let url = console_url(&project, context.account.as_ref());
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
