# hop

Fast, interactive context switching for Google Cloud Platform. Think [granted.dev](https://granted.dev), but for GCP.

hop switches by updating gcloud's own configuration state, so the change is global: every open terminal, `gcloud` itself, and prompt tooling (like starship's gcloud module) reflect it immediately. No `eval`, no shell hooks, no per-shell state.

## Status

Early development (pre-0.1). Working today:

- `hop status`: show the active configuration, account, project, and impersonation state
- `hop switch [name]`: switch the active gcloud configuration and project, interactively or fully by flags
- `hop login [account]`: authenticate via gcloud's browser flow
- Automatic detection of expired credentials on switch, with a configurable re-auth prompt

Planned (see the milestone plan): SSO / workforce identity login, `hop console`, `hop impersonate`.

## Install

### From source (all platforms)

Requires Rust 1.87 or newer.

```sh
git clone https://github.com/mervinhemaraju/hop
cd hop
cargo install --path .
```

### Homebrew (macOS / Linux)

Coming with the first tagged release.

### Windows

Builds and runs from source today (`cargo install --path .`); packaged distribution comes after the Homebrew release.

## Shell setup

None. Because hop writes gcloud's own global state, there is nothing to source and no shell integration to install.

## Usage

### `hop status`

Show the active context. Works offline; reads only local gcloud files.

```sh
$ hop status
config directory:     /home/me/.config/gcloud
active configuration: work
account:              dev@example.com
project:              my-project-123
impersonation:        (not set)
```

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

| Code | Meaning |
|------|---------|
| 0    | switched, or already on the target |
| 1    | could not read or write gcloud state, or a GCP API call failed |
| 2    | unknown configuration name, or invalid project id |
| 3    | interactive picker needed but no terminal available |
| 4    | credentials expired or revoked (run `hop login`) |
| 130  | cancelled from the configuration picker (Esc or Ctrl+C) |

All human-facing output goes to stderr; stdout is reserved for future machine-consumable output, so hop is safe to pipe.

### `hop login [account]`

Authenticate a Google account through gcloud's browser flow (hop never handles your password or stores keys; gcloud owns the flow end to end).

```sh
hop login                        # authenticate a new or existing account
hop login dev@example.com        # re-authenticate a specific account
hop login --no-launch-browser    # print the auth URL instead (SSH sessions)
```

Exit codes: `0` success, `1` login failed or gcloud unavailable, `2` invalid account.

## Settings

Optional, at `~/.config/hop/settings.json` (Linux/macOS) or `%APPDATA%\hop\settings.json` (Windows):

```json
{
  "reauth": "prompt"
}
```

`reauth` controls what a switch does when it meets expired credentials: `"prompt"` (default) asks first, `"auto"` runs the login flow immediately, `"off"` never logs in and fails with exit code 4.

The project cache lives next to it under `cache/`; delete it freely, `--refresh` rebuilds it.

## How impersonation will work

Not implemented yet. The design: hop sets the `auth/impersonate_service_account` property on the active configuration and mints short-lived tokens via the IAM Credentials API. No service account key files are ever written to disk.

## Troubleshooting

- **`no gcloud configurations found`**: gcloud has never been initialised on this machine; run `gcloud init` or `gcloud config configurations create <name>` first.
- **`credentials ... are expired or revoked`** (exit code 4): run `hop login <account>`; hop delegates entirely to gcloud for authentication.
- **The project picker shows a stale list**: pass `--refresh`, or delete hop's `cache/` directory.
- **`no active projects visible to this account`**: the account lacks `resourcemanager.projects.get` on any active project; check with `gcloud projects list`.
- **hop looks at the wrong directory**: hop honours `CLOUDSDK_CONFIG`, the same override gcloud uses (and `HOP_CONFIG` for its own settings/cache). Unset them or point them at the right place.
- **No colors wanted**: set `NO_COLOR=1` (see [no-color.org](https://no-color.org)); color is also disabled automatically when output is not a terminal.
- **`failed to parse .../config_<name>`**: the configuration file is not in the format gcloud writes. Fix it by hand or recreate it with `gcloud config configurations create`.
