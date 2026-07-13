# Usage

Every interactive flow has a non-interactive equivalent, so everything here
is scriptable. Human-facing output goes to stderr; stdout is reserved for
machine-consumable output (`--url`, future JSON), so hop is always safe to
pipe.

## `hop` / `hop switch [name]`

Switch the active gcloud configuration, then pick a project for it. Bare
`hop` is a shortcut for `hop switch`.

```sh
hop                                         # pick a configuration, then a project
hop switch work                             # switch configuration, then pick a project
hop switch work --project my-project-123    # fully non-interactive
hop switch work --refresh                   # re-fetch the project list from GCP
```

The switch is global: it updates gcloud's own active configuration, so
every open terminal, `gcloud` itself, and prompt tooling (starship's gcloud
module, for example) reflect it immediately.

Projects are fetched from the Cloud Resource Manager API once and cached
per account, so the picker opens instantly and works offline afterwards.
Pass `--refresh` after creating or gaining access to new projects. Pressing
Esc at the project picker keeps the configuration switch and leaves the
project as it was.

If the account's credentials are expired, hop offers to run the login flow
right there (the `reauth` setting controls this; see
[Configuration](configuration.md)).

Exit codes:

| Code | Meaning                                                        |
|------|----------------------------------------------------------------|
| 0    | switched, or already on the target                             |
| 1    | could not read or write gcloud state, or a GCP API call failed |
| 2    | unknown configuration name, or invalid project id              |
| 3    | interactive picker needed but no terminal available            |
| 4    | credentials expired or revoked (run `hop login`)               |
| 130  | cancelled from the configuration picker (Esc or Ctrl+C)        |

## `hop status`

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

Workforce (SSO) sessions show `identity: workforce federation` and the
`principal://...` identifier as the account.

## `hop login [account]`

Authenticate through gcloud. hop never handles your password or stores
keys; gcloud owns the flow end to end.

```sh
hop login                             # authenticate a new or existing Google account
hop login dev@example.com             # re-authenticate a specific account
hop login --sso                       # SSO via the active configuration's login config
hop login --login-config wf.json      # SSO via an explicit login config file
hop login --no-launch-browser         # print the auth URL instead (SSH sessions)
```

`--sso` reads the `auth/login_config_file` property from the active
configuration; `--login-config` overrides it for one run. See
[Configuration](configuration.md) for the one-time SSO setup.

The browser is chosen from the `BROWSER` environment variable, then the
`browser` setting, then the system default.

Exit codes: `0` success, `1` login failed, gcloud unavailable, or no login
config found for `--sso`, `2` invalid account or missing `--login-config`
file.

## `hop console [--project X] [--url]`

Open the GCP console for the active context. The URL carries
`authuser=<active account>`, so the right Google session opens even with
several accounts signed in. Workforce sessions get the federated console
sign-in instead of the standard one.

```sh
hop console                           # active project's dashboard
hop console --project my-project-123  # a specific project
hop console --url                     # print the URL to stdout (no browser)
```

`--url` writes to stdout and nothing else, so it composes:
`open "$(hop console --url)"`.

Exit codes: `0` opened, `1` no project set or the browser failed,
`2` invalid project id.

## `hop impersonate [sa] [--clear]`

Set service account impersonation on the active configuration (gcloud's
`auth/impersonate_service_account` property, which all terminals see
immediately).

```sh
hop impersonate        # pick from the active project's service accounts
hop impersonate deploy@my-project-123.iam.gserviceaccount.com
hop impersonate --clear
```

Before writing anything, hop proves the impersonation works by minting a
short-lived token via the IAM Credentials API and discarding it. A missing
`roles/iam.serviceAccountTokenCreator` on the target fails immediately with
exit code `5`, instead of silently breaking every later gcloud call.

How it works: from the moment the property is set, gcloud makes API
requests as that service account, with tokens minted on demand. No key
files are ever written, and hop never prints or stores token material.
`--clear` removes the property and returns you to your own account.

Exit codes: as `hop switch`, plus `5` for permission denied while minting.

## Troubleshooting

See the [main README](../../README.md#troubleshooting) for the
troubleshooting list; it covers the common errors, their exit codes, and
the fix for each.
