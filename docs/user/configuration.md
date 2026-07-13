# Configuration

hop works on top of two kinds of configuration: gcloud's own state (the
configurations you switch between) and hop's small optional settings file.

## gcloud configurations

A gcloud *configuration* is a named profile binding an account, a project,
and other properties. hop's picker lists them; `hop switch` activates them.
Create one per context you work in:

```sh
gcloud config configurations create work
hop login dev@example.com          # authenticate and bind the account

gcloud config configurations create personal
hop login me@example.com
```

You do not need to set a project by hand; `hop switch` presents a project
picker and writes your choice into the configuration. To see everything
gcloud has:

```sh
gcloud config configurations list
```

## SSO (workforce identity federation)

If an organization signs you in through an IdP (Okta, Entra ID, ...) using
[workforce identity federation](https://docs.cloud.google.com/iam/docs/workforce-identity-federation),
attach a *login config file* to the configuration once:

```sh
gcloud config configurations create customer

gcloud iam workforce-pools create-login-config \
    locations/global/workforcePools/my-pool/providers/my-okta \
    --output-file="$HOME/.gcp/customer/login-config.json" \
    --activate
```

`--activate` sets the `auth/login_config_file` property on the active
configuration. That property makes everything automatic afterwards:
`hop login --sso` finds the file by itself, expired-credential re-auth uses
the SSO flow, and `hop console` opens the federated console.

Already have a login config JSON from an admin? Point the configuration at
it instead of generating one:

```sh
gcloud config set auth/login_config_file /path/to/login-config.json
```

Then sign in with `hop login --sso`. When the session expires (typically
after an hour), the same short command re-authenticates; the file path
never needs repeating.

## hop's settings file

Optional. hop reads:

- Linux / macOS: `~/.config/hop/settings.json`
- Windows: `%APPDATA%\hop\settings.json`

Create the file yourself; every key is optional:

```json
{
  "reauth": "prompt",
  "browser": "~/scripts/open-work-browser"
}
```

### `reauth`

What `hop switch` and `hop impersonate` do when they meet expired
credentials:

| Value      | Behaviour                                              |
|------------|--------------------------------------------------------|
| `"prompt"` | Ask before launching the login flow (default)          |
| `"auto"`   | Launch the login flow immediately, no question         |
| `"off"`    | Never log in; fail with exit code 4 (`hop login` yourself) |

### `browser`

The command hop uses to open URLs, for both login flows and `hop console`.
Unset, the system default browser opens. The command is invoked with the
URL as its only argument, so anything needing more arguments (a specific
profile, a container tab) goes in a small wrapper script:

```sh
#!/usr/bin/env bash
# ~/scripts/open-work-browser
exec /Applications/Firefox.app/Contents/MacOS/firefox -P work "$1"
```

A leading `~/` in the value is expanded. The `BROWSER` environment variable
overrides the setting for a single run.

## Environment variables

| Variable          | Effect                                                              |
|-------------------|---------------------------------------------------------------------|
| `BROWSER`         | Overrides the `browser` setting for one invocation                  |
| `CLOUDSDK_CONFIG` | Points hop (and gcloud) at a different gcloud config directory      |
| `HOP_CONFIG`      | Points hop at a different directory for its own settings and cache  |
| `NO_COLOR`        | Disables colored output (also automatic when output is not a TTY)   |

## Files hop touches

- gcloud's configuration files (read always; written only on explicit
  switch/impersonate actions)
- `~/.config/hop/settings.json` (read only; you create and edit it)
- `~/.config/hop/cache/projects-<account>.json` (project list cache,
  owner-only permissions; delete freely, `hop switch --refresh` rebuilds it)

hop never writes credentials or key files, and nothing leaves your machine
except the GCP API calls you trigger (project listing, impersonation
verification).
