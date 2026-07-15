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
    /// Authenticate via gcloud: Google account or SSO (workforce identity)
    #[command(after_long_help = "\
Examples:
  hop login                             authenticate a Google account
  hop login dev@example.com             re-authenticate a specific account
  hop login --sso                       SSO via the configured workforce login config
  hop login --login-config wf.json      SSO via an explicit login config file
  hop login --no-launch-browser         print the auth URL instead of opening a browser

--sso uses the auth/login_config_file property from the active configuration
(set by `gcloud iam workforce-pools create-login-config <provider> --activate`).

The browser is chosen from the BROWSER environment variable, then the
\"browser\" setting in hop's settings.json, then the system default.

Exit codes: 0 success, 1 login failed, gcloud unavailable, or no login config
found for --sso, 2 invalid account or missing --login-config file.")]
    Login {
        /// Account email to authenticate, e.g. dev@example.com
        account: Option<String>,
        /// Sign in with SSO using the active configuration's workforce login config
        #[arg(long)]
        sso: bool,
        /// Workforce login config file to use (implies SSO), e.g. wf-login.json
        #[arg(long)]
        login_config: Option<String>,
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

The configuration picker hides each entry's account/principal by default;
pass --show-principal to reveal it.

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
        /// Show each configuration's account/principal in the picker
        #[arg(long)]
        show_principal: bool,
    },
    /// Open the GCP console in the browser for a chosen configuration and project
    #[command(after_long_help = "\
Examples:
  hop console                          pick a configuration, then a project, then open
  hop console work                     use configuration `work`, then pick a project
  hop console work --project my-project-123   no pickers, open directly
  hop console --refresh                refresh the cached project list first
  hop console --url                    print the URL to stdout instead

On a terminal, hop lists your configurations then your projects (the same
pickers as `hop switch`) and opens the console for what you choose. Unlike
`hop switch`, console never changes your active gcloud configuration: it only
reads the chosen one's account and identity to open the console. Without a
terminal (e.g. piped), it uses the active configuration and its project so
scripts keep working. Projects are cached locally; pass --refresh after
creating new ones.

The configuration picker hides each entry's account/principal by default;
pass --show-principal to reveal it.

The URL pins the console to the chosen configuration's account (authuser), so
the right Google session opens even with multiple accounts signed in.

The browser is chosen from the BROWSER environment variable, then the
\"browser\" setting in hop's settings.json, then the system default.

Exit codes:
  0    opened (or URL printed)
  1    no project available or the browser failed to open
  2    unknown configuration name or invalid project id
  4    credentials expired or revoked (run `hop login`)
  130  cancelled from a picker (Esc or Ctrl+C)")]
    Console {
        /// Configuration to open the console with (skips the picker), e.g. work
        name: Option<String>,
        /// Open this project instead of picking one, e.g. my-project-123
        #[arg(long)]
        project: Option<String>,
        /// Print the console URL to stdout instead of opening the browser
        #[arg(long)]
        url: bool,
        /// Re-fetch the project list from GCP instead of using the local cache
        #[arg(long)]
        refresh: bool,
        /// Show each configuration's account/principal in the picker
        #[arg(long)]
        show_principal: bool,
    },
    /// Impersonate a service account on the active configuration
    #[command(after_long_help = "\
Examples:
  hop impersonate                      pick from the active project's service accounts
  hop impersonate deploy@my-project-123.iam.gserviceaccount.com
  hop impersonate --clear              stop impersonating

hop verifies impersonation by minting (and discarding) a short-lived token
before writing anything, so a missing role fails immediately. Requires
roles/iam.serviceAccountTokenCreator on the target service account.

Exit codes:
  0    impersonation set (verified) or cleared
  1    could not read/write gcloud state or an API call failed
  2    invalid service account email
  3    interactive picker needed but no terminal available
  4    credentials expired or revoked (run `hop login`)
  5    permission denied minting a token (missing token-creator role)
  130  cancelled (Esc, Ctrl+C, or declining the re-auth prompt)")]
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
