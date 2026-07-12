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
    /// Switch the active gcloud configuration
    #[command(after_long_help = "\
Examples:
  hop switch          open the interactive picker (arrow keys, type to filter)
  hop switch work     switch to the configuration named `work` directly

The switch is global: it updates gcloud's own active configuration, so every
terminal and prompt tool reflects it immediately.

Exit codes:
  0    switched (or already on the target)
  1    could not read or write gcloud state
  2    no configuration with the given name
  3    interactive picker needed but no terminal available
  130  cancelled from the picker (Esc or Ctrl+C)")]
    Switch {
        /// Configuration name to switch to (skips the picker), e.g. work
        name: Option<String>,
    },
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
