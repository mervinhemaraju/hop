# hop

Fast, interactive context switching for Google Cloud Platform. Think [granted.dev](https://granted.dev), but for GCP.

hop switches by updating gcloud's own configuration state, so the change is global: every open terminal, `gcloud` itself, and prompt tooling (like starship's gcloud module) reflect it immediately. No `eval`, no shell hooks, no per-shell state.

## Status

Early development (pre-0.1). Working today:

- `hop status`: show the active configuration, account, project, and impersonation state
- `hop switch [name]`: switch the active gcloud configuration, interactively or by name

Planned (see the milestone plan): `hop login` (including SSO / workforce identity), project switching with an interactive picker, `hop console`, `hop impersonate`.

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

Switch the active gcloud configuration.

```sh
hop switch          # interactive picker: arrow keys to move, type to fuzzy-filter
hop switch work     # switch directly, no prompt (script-friendly)
hop                 # bare `hop` is a shortcut for `hop switch`
```

Exit codes, for script authors:

| Code | Meaning |
|------|---------|
| 0    | switched, or already on the target |
| 1    | could not read or write gcloud state |
| 2    | no configuration with the given name |
| 3    | interactive picker needed but no terminal available |
| 130  | cancelled from the picker (Esc or Ctrl+C) |

All human-facing output goes to stderr; stdout is reserved for future machine-consumable output, so hop is safe to pipe.

## How impersonation will work

Not implemented yet. The design: hop sets the `auth/impersonate_service_account` property on the active configuration and mints short-lived tokens via the IAM Credentials API. No service account key files are ever written to disk.

## Troubleshooting

- **`no gcloud configurations found`**: gcloud has never been initialised on this machine; run `gcloud init` or `gcloud config configurations create <name>` first.
- **hop looks at the wrong directory**: hop honours `CLOUDSDK_CONFIG`, the same override gcloud uses. Unset it or point it at the right place.
- **No colors wanted**: set `NO_COLOR=1` (see [no-color.org](https://no-color.org)); color is also disabled automatically when output is not a terminal.
- **`failed to parse .../config_<name>`**: the configuration file is not in the format gcloud writes. Fix it by hand or recreate it with `gcloud config configurations create`.
