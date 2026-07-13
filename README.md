# hop

Fast, interactive context switching for Google Cloud Platform. Think [granted.dev](https://granted.dev), but for GCP.

If you work across several GCP projects, accounts, or customer environments, you know the dance: `gcloud config configurations activate`, `gcloud config set project`, re-authenticating expired sessions, hunting for the right console tab. hop replaces that with one short command and an arrow-key picker.

```sh
$ hop
? Configuration:
> work      dev@example.com        my-project-123 (active)
  personal  me@example.com         hobby-project-456
  customer  principal://iam.googleapis.com/...

? Project:
> my-project-123    My Project
  my-staging-123    My Staging
```

## What hop does

- **Switch context in one step**: pick a gcloud configuration and a project with arrow keys, or pass them as arguments for scripts
- **Global switching**: hop updates gcloud's own configuration state, so the change is visible everywhere at once; every open terminal, `gcloud` itself, and prompt tooling (like starship's gcloud module) reflect it immediately. No `eval`, no shell hooks, no per-shell state.
- **Authentication built in**: launches gcloud's login flows, detects expired credentials on switch, and offers to re-authenticate on the spot
- **SSO / workforce identity federation**: sign in through your organization's IdP, with the right federated console URLs and re-auth flows handled automatically
- **Service account impersonation**: short-lived tokens only, verified before anything is written; no key files, ever
- **Console in the right session**: `hop console` opens the GCP console pinned to the active account, even with several Google accounts signed in

hop never creates, modifies, or deletes cloud resources. It is a switcher, not a management tool: the only thing it touches is your local gcloud context, plus minting short-lived tokens when you ask for impersonation.

## Requirements

- The [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) (`gcloud`) installed and on your `PATH`; hop delegates all login flows to it and reads its configuration files
- macOS, Linux, or Windows

## Install

### Homebrew (macOS / Linux)

```sh
brew install mervinhemaraju/hop/hop
```

This taps `mervinhemaraju/homebrew-hop` automatically. Available from the first tagged release (v0.1.0); until then, install from source.

### From source (all platforms)

Requires Rust 1.87 or newer.

```sh
git clone https://github.com/mervinhemaraju/hop
cd hop
cargo install --path .
```

### Windows

Builds and runs from source today (`cargo install --path .`); packaged distribution (winget/Scoop) comes after the Homebrew release.

## Shell setup

None. Because hop writes gcloud's own global state, there is nothing to source and no shell integration to install.

## Getting started

hop works with gcloud *configurations*: named profiles that each bind an account, a project, and other properties. If you already have configurations set up, skip ahead to [Usage](#usage); `hop status` will show what hop sees.

### 1. Create a configuration per context

One configuration per account or environment you switch between:

```sh
gcloud config configurations create work
hop login dev@example.com         # authenticate and bind the account

gcloud config configurations create personal
hop login me@example.com
```

You do not need to set a project here; `hop switch` presents a project picker and writes your choice to the configuration.

### 2. Optional: set up SSO (workforce identity federation)

If your organization signs in through an IdP (Okta, Entra ID, ...) using [workforce identity federation](https://docs.cloud.google.com/iam/docs/workforce-identity-federation), create a login config file once and attach it to a configuration:

```sh
gcloud config configurations create customer

gcloud iam workforce-pools create-login-config \
    locations/global/workforcePools/my-pool/providers/my-okta \
    --output-file="$HOME/.gcp/customer/login-config.json" \
    --activate
```

`--activate` sets the `auth/login_config_file` property on the active configuration. That property is what makes everything automatic afterwards: `hop login --sso` finds the file by itself, expired-credential re-auth uses the SSO flow, and `hop console` opens the federated console.

Already have a login config JSON from your admin? Point the configuration at it instead of generating one:

```sh
gcloud config set auth/login_config_file /path/to/login-config.json
```

Then sign in:

```sh
hop login --sso
```

When the session expires (typically after an hour), the same short `hop login --sso` re-authenticates; no need to repeat the file path. hop's switch and impersonate commands also offer to run it for you when they hit expired credentials.

### 3. Hop around

```sh
hop            # bare hop = hop switch: pick a configuration, then a project
hop status     # see where you are
hop console    # open the GCP console for the active context
```

## Usage

### `hop status`

Show the active context. Works offline; reads only local gcloud files.

```sh
$ hop status
config directory:     /home/me/.config/gcloud
active configuration: work
identity:             Google account
account:              dev@example.com
project:              my-project-123
impersonation:        (not set)
```

Workforce (SSO) sessions show `identity: workforce federation` and the `principal://...` identifier as the account.

### `hop switch [name]`

Switch the active gcloud configuration, then pick a project for it.

```sh
hop switch                                  # pick a configuration, then a project
hop switch work                             # switch configuration, then pick a project
hop switch work --project my-project-123    # fully non-interactive (script-friendly)
hop switch work --refresh                   # also re-fetch the project list from GCP
hop                                         # bare `hop` is a shortcut for `hop switch`
```

Projects are fetched from the Cloud Resource Manager API once and cached locally, so the picker opens instantly and works offline afterwards. Pass `--refresh` after creating or gaining access to new projects. Pressing Esc at the project picker keeps the configuration switch and leaves the project as it was.

If the account's credentials turn out to be expired, hop offers to run the login flow right there (see [Settings](#settings) to make that automatic or turn it off).

Exit codes, for script authors:

| Code | Meaning                                                        |
|------|----------------------------------------------------------------|
| 0    | switched, or already on the target                             |
| 1    | could not read or write gcloud state, or a GCP API call failed |
| 2    | unknown configuration name, or invalid project id              |
| 3    | interactive picker needed but no terminal available            |
| 4    | credentials expired or revoked (run `hop login`)               |
| 130  | cancelled from the configuration picker (Esc or Ctrl+C)        |

All human-facing output goes to stderr; stdout is reserved for machine-consumable output, so hop is safe to pipe.

### `hop login [account]`

Authenticate through gcloud (hop never handles your password or stores keys; gcloud owns the flow end to end).

```sh
hop login                             # authenticate a new or existing Google account
hop login dev@example.com             # re-authenticate a specific account
hop login --sso                       # SSO via the active configuration's login config
hop login --login-config wf.json      # SSO via an explicit login config file
hop login --no-launch-browser         # print the auth URL instead (SSH sessions)
```

`--sso` reads the `auth/login_config_file` property from the active configuration (see [Getting started](#2-optional-set-up-sso-workforce-identity-federation)); `--login-config` overrides it for one run.

The browser is chosen from the `BROWSER` environment variable, then the `browser` [setting](#settings), then the system default.

Exit codes: `0` success, `1` login failed, gcloud unavailable, or no login config found for `--sso`, `2` invalid account or missing `--login-config` file.

### `hop console [--project X] [--url]`

Open the GCP console for the active context. The URL carries `authuser=<active account>`, so the right Google session opens even when several accounts are signed in. Workforce sessions get the federated console (`auth.cloud.google` sign-in) instead of the standard one.

```sh
hop console                           # active project's dashboard
hop console --project my-project-123  # a specific project
hop console --url                     # print the URL to stdout (no browser)
```

`--url` writes to stdout and nothing else, so it composes: `open "$(hop console --url)"`. Exit codes: `0` opened, `1` no project set or the browser failed, `2` invalid project id.

### `hop impersonate [sa] [--clear]`

Set service account impersonation on the active configuration (gcloud's `auth/impersonate_service_account` property, which all terminals see immediately).

```sh
hop impersonate        # pick from the active project's service accounts
hop impersonate deploy@my-project-123.iam.gserviceaccount.com
hop impersonate --clear
```

Before writing anything, hop proves the impersonation works by minting a short-lived token via the IAM Credentials API and discarding it. A missing `roles/iam.serviceAccountTokenCreator` on the target fails right there with exit code `5`, instead of silently breaking every later gcloud call. Other exit codes match `hop switch` (`4` expired credentials, `130` cancelled).

## Settings

Optional. hop reads its settings from:

- Linux / macOS: `~/.config/hop/settings.json`
- Windows: `%APPDATA%\hop\settings.json`

Create the file yourself; all keys are optional:

```json
{
  "reauth": "prompt",
  "browser": "~/scripts/open-work-browser"
}
```

`reauth` controls what a switch does when it meets expired credentials: `"prompt"` (default) asks first, `"auto"` runs the login flow immediately, `"off"` never logs in and fails with exit code 4.

`browser` sets the command hop uses to open URLs, both for login flows and `hop console`; unset, the system default browser opens. The command is invoked with the URL as its only argument, so anything needing more arguments (a specific profile, a container tab) goes in a small wrapper script. A leading `~/` is expanded. The `BROWSER` environment variable overrides the setting for a single run.

The project cache lives next to the settings under `cache/`; delete it freely, `--refresh` rebuilds it.

## How impersonation works

hop sets the `auth/impersonate_service_account` property on the active configuration; from then on gcloud makes API requests as that service account. Tokens are short-lived and minted on demand via the IAM Credentials API. No service account key files are ever written to disk, and hop never prints or stores token material.

## Troubleshooting

- **`no gcloud configurations found`**: gcloud has never been initialised on this machine; run `gcloud init` or `gcloud config configurations create <name>` first.
- **`credentials ... are expired or revoked`** (exit code 4): run `hop login <account>` (or `hop login --sso` for workforce sessions); hop delegates entirely to gcloud for authentication.
- **`--sso needs a workforce login config`**: the active configuration has no `auth/login_config_file` property; set one as shown in [Getting started](#2-optional-set-up-sso-workforce-identity-federation), or pass `--login-config <file>`.
- **The project picker shows a stale list**: pass `--refresh`, or delete hop's `cache/` directory.
- **`no active projects visible to this account`**: the account lacks `resourcemanager.projects.get` on any active project; check with `gcloud projects list`.
- **`permission denied` when impersonating** (exit code 5): grant yourself `roles/iam.serviceAccountTokenCreator` on the target service account (an admin does this once; propagation can take a minute).
- **URLs open in the wrong browser or profile**: set the `browser` key in [settings](#settings) to a wrapper script that launches the browser you want.
- **hop looks at the wrong directory**: hop honours `CLOUDSDK_CONFIG`, the same override gcloud uses (and `HOP_CONFIG` for its own settings/cache). Unset them or point them at the right place.
- **No colors wanted**: set `NO_COLOR=1` (see [no-color.org](https://no-color.org)); color is also disabled automatically when output is not a terminal.
- **`failed to parse .../config_<name>`**: the configuration file is not in the format gcloud writes. Fix it by hand or recreate it with `gcloud config configurations create`.
