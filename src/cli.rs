//! clap definitions only; no logic lives here (rules/architecture.md).

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "hop",
    version,
    about = "Fast, interactive context switching for Google Cloud Platform"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Authenticate a Google account via gcloud's browser flow
    Login {
        /// Account email to authenticate, e.g. dev@example.com
        account: Option<String>,
    },
    /// Switch the active account and project interactively
    Switch,
    /// Open the GCP console in the browser for the active context
    Console {
        /// Open this project instead of the active one, e.g. my-project-123
        #[arg(long)]
        project: Option<String>,
    },
    /// Impersonate a service account on the active configuration
    Impersonate {
        /// Service account email, e.g. deploy@my-project-123.iam.gserviceaccount.com
        service_account: Option<String>,
        /// Stop impersonating and return to your own account
        #[arg(long, conflicts_with = "service_account")]
        clear: bool,
    },
    /// Show the active context (configuration, account, project, impersonation)
    Status,
}
