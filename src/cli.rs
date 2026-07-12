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
    #[command(after_long_help = "\
Examples:
  hop login                        authenticate a new or existing account
  hop login dev@example.com        re-authenticate a specific account
  hop login --no-launch-browser    print the auth URL instead of opening a browser

Exit codes: 0 success, 1 login failed or gcloud unavailable, 2 invalid account.")]
    Login {
        /// Account email to authenticate, e.g. dev@example.com
        account: Option<String>,
        /// Print the authorization URL instead of opening a browser (for SSH sessions)
        #[arg(long)]
        no_launch_browser: bool,
    },
    /// Switch the active gcloud configuration and project
    #[command(after_long_help = "\
Examples:
  hop switch                                  pick a configuration, then a project
  hop switch work                             switch configuration, then pick a project
  hop switch work --project my-project-123    fully non-interactive switch
  hop switch work --refresh                   refresh the cached project list too

Projects are listed from the Cloud Resource Manager API and cached locally,
so the picker opens instantly; pass --refresh after creating new projects.
Pressing Esc at the project picker keeps the configuration switch.

The switch is global: it updates gcloud's own active configuration, so every
terminal and prompt tool reflects it immediately.

Exit codes:
  0    switched (or already on the target)
  1    could not read or write gcloud state, or a GCP API call failed
  2    unknown configuration name or invalid project id
  3    interactive picker needed but no terminal available
  4    credentials expired or revoked (run `hop login`)
  130  cancelled from the configuration picker (Esc or Ctrl+C)")]
    Switch {
        /// Configuration name to switch to (skips the picker), e.g. work
        name: Option<String>,
        /// Project id to set on the configuration (skips the project picker), e.g. my-project-123
        #[arg(long)]
        project: Option<String>,
        /// Re-fetch the project list from GCP instead of using the local cache
        #[arg(long)]
        refresh: bool,
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
