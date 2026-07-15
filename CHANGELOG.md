# Changelog

All notable changes to hop are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and hop adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-15

### Added

- `hop console` now opens an interactive picker flow like `hop switch`: choose
  a configuration, then a project, then it opens that project's console. The
  configuration name skips the config picker, `--project` skips the project
  picker, and `--refresh` re-fetches the project list. Without a terminal
  (e.g. piped), it uses the active configuration and its project so scripts
  keep working. Console is read-only: it never changes your active gcloud
  configuration, only reads the chosen one's account and identity.
- `--show-principal` on `hop switch` and `hop console`: the configuration
  picker hides each entry's account/principal by default and reveals it with
  this flag.

### Changed

- The configuration picker label is now `name  project  (active)`; the
  account/principal is hidden unless `--show-principal` is passed.

### Fixed

- Re-authentication for workforce (SSO) sessions. When credentials expired,
  hop passed the `principal://…` account as a positional argument to
  `gcloud auth login` alongside `--login-config`, so gcloud tried to refresh
  the expired token (`invalid_grant`) instead of a fresh federated sign-in.
  hop now omits the account positional whenever a login config is present, so
  `switch`, `console`, and `impersonate` re-authenticate correctly for
  workforce identities.

## [0.1.0] - 2026-07-13

First public release.

### Added

- `hop switch` (or bare `hop`): switch the active gcloud configuration and
  project with an arrow-key picker, or fully non-interactively with
  `hop switch <name> --project <id>`. Switching is global: every terminal,
  `gcloud` itself, and prompt tooling see the change immediately.
- Project listings fetched from the Cloud Resource Manager API and cached
  per account, so the picker opens instantly and works offline; `--refresh`
  re-fetches.
- `hop status`: show the active configuration, identity type, account,
  project, and impersonation state; works fully offline.
- `hop login`: authenticate through gcloud, with `--no-launch-browser` for
  SSH sessions.
- SSO via workforce identity federation: `hop login --sso` (uses the active
  configuration's login config) and `hop login --login-config <file>`;
  `hop status` shows the federated identity.
- `hop console`: open the GCP console for the active context, pinned to the
  active account; workforce sessions get the federated console sign-in.
  `--project` overrides the project, `--url` prints the URL to stdout
  instead of opening a browser.
- `hop impersonate`: set service account impersonation, picking from the
  active project's service accounts or passing an email directly;
  impersonation is verified by minting a short-lived token before anything
  is written. `--clear` stops impersonating.
- Expired-credential detection on switch and impersonate, with a
  configurable re-auth policy (`prompt`, `auto`, or `off`) in
  `settings.json`.
- `browser` setting: open login flows and the console through a custom
  browser command; the `BROWSER` environment variable overrides it per run.
- Script-friendly behaviour throughout: distinct exit codes per failure
  class (0, 1, 2, 3, 4, 5, 130), human output on stderr only, stdout
  reserved for machine-consumable output, `NO_COLOR` respected.

### Security

- Impersonation uses short-lived tokens minted via the IAM Credentials API;
  no service account key files are ever written to disk.
- Token material is never printed, logged, or included in error messages.
- hop's cache files are created with owner-only (0600) permissions on Unix.
- No telemetry, no update checks, no data leaves your machine.

[0.2.0]: https://github.com/mervinhemaraju/hop/releases/tag/v0.2.0
[0.1.0]: https://github.com/mervinhemaraju/hop/releases/tag/v0.1.0
