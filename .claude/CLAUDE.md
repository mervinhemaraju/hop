# hop

## What This Is

`hop` is a CLI tool written in Rust for fast, interactive context switching on Google Cloud Platform; think [granted.dev](https://granted.dev), but for GCP instead of AWS.

Core capabilities (planned):

- Switch between multiple GCP projects and service account impersonations within an account, interactively or by name (impersonation via short-lived credentials, no key files)
- Switch between multiple authenticated gcloud accounts
- On switch, detect expired credentials and prompt to re-authenticate (like granted.dev does); a user setting can make this fully automatic or turn it off
- Keep the active context visible and easy to inspect

The architecture and command surface are not finalized yet. Do not invent commands, flags, or module layouts beyond what exists in the code; when the design is ambiguous, ask before assuming.

## Status

Phases 1-3 implemented: layered structure (cli/commands/core/adapters), validated newtypes (incl. redacted-Debug `AccessToken`), ports + fakes, defensive gcloud INI parser (read + byte-preserving write), `hop status` (full context), `hop login` (delegates to `gcloud auth login`), and full `hop switch`: configuration picker then project picker (Resource Manager v3 `projects:search` via ureq, per-account 0600 cache, `--refresh`), non-interactive forms (`<name>`, `--project`), expired-credential detection via `gcloud auth print-access-token` exit status with prompt/auto/off reauth policy in `~/.config/hop/settings.json`. Exit codes 0/1/2/3/4/130. Dependencies: clap, thiserror, inquire, serde, serde_json, ureq (no default features; cookies off) — all vetted via /add-crate, cargo audit clean.

The milestone plan is agreed and lives at `.claude/PLAN.md` (7 phases: skeleton, local context read/write, auth + projects, console + impersonation, SSO / workforce identity, Homebrew release, post-release improvements backlog). Key decisions already made:

- **Backend: hybrid.** gcloud binary for login flows only; gcloud config files read directly; direct API calls for project listing and impersonation; console via URL building.
- **Switching: global state.** hop writes gcloud's own active configuration so all terminals and prompt tooling (starship) reflect the change. No per-shell env-var/eval pattern.

Do not start a phase without the user explicitly saying to begin it. Plan approval alone is not the signal to write code.

**Working mode (user decision, 2026-07-11): step-by-step delivery.** Implement in small runnable increments, not whole phases in one go. After each increment: show what was built, give the user the exact commands to run it themselves, and wait for their approval before the next increment. Keep the "Current step" line below up to date.

Current step: Phase 5 (SSO/workforce) COMPLETE, user-confirmed 2026-07-13 (real-IdP round-trip done by user against their workforce tenant with a login-config file + custom browser script). Built in Phase 5: `core/workforce.rs` (audience parsing), `Context::identity()` (principal:// prefix detection), `hop login --sso/--login-config`, federated `hop console` (`auth.cloud.google` sign-in URL), identity line in `hop status`, workforce-aware re-auth (login config threaded through auth_flow). Side feature (2026-07-13, user-requested, user-tested OK): `browser` setting in settings.json (custom browser command for login flows and `hop console`; BROWSER env var overrides; `~/` expanded; `GcloudCli::new(Option<PathBuf>)` sets BROWSER on the gcloud child, `CustomBrowser` adapter spawns the command with the URL as sole arg, `core::settings::effective_browser` holds the precedence). 98 tests green. README fully restructured 2026-07-13 (intro/features, install, Getting started covering gcloud configuration creation + SSO login-config setup, usage per command, settings paths, troubleshooting). Phase 6 IN PROGRESS (2026-07-13): tap decision = single `mervinhemaraju/homebrew-tap` repo holding all formulas (install: `brew install mervinhemaraju/tap/hop`); added LICENSE (MIT), `.github/workflows/ci.yml` (fmt/clippy/test on macOS+Linux, compile-only Windows), `.github/workflows/release.yml` (on `v*` tag: version-check vs Cargo.toml, builds aarch64/x86_64 darwin + x86_64 musl tar.gz + windows zip, SHA256SUMS, drafts the GitHub release for the user to publish; third-party actions SHA-pinned, checkout persist-credentials off per SAST). Remaining: user pushes + CI green, user runs /release-prep 0.1.0 (changelog, formula draft for the tap), user tags v0.1.0, publishes draft release, creates homebrew-tap repo + pushes formula, verifies `brew install` on a clean machine. Next after that: Phase 7 backlog.

Previous: Phase 4 implemented on 2026-07-12 (whole-phase mode, per user), /rust-review passed (2 findings fixed: URL path encoding, stale dead-code allowance). `/gcp-check` done first: `generateAccessToken` (POST v1 projects/-/serviceAccounts/{email}, needs iam.serviceAccounts.getAccessToken), serviceAccounts.list (pageSize max 100), `auth/impersonate_service_account` semantics doc-quoted, authuser URL param confirmed via Cloud Shell docs. Built: `hop console [--project] [--url]` (core URL builder + open crate), `hop impersonate [sa] [--clear]` (verify-mint before write; minted token never read from the response body; clear removes the INI line via new remove_property), shared commands/auth_flow.rs (reauth policy, used by switch + impersonate), exit code 5 = permission denied. 80 tests green; e2e on fake dirs (console URLs, clear). open 5.4.0 vetted; audit clean. User still to test: real impersonation against a sandbox SA, interactive SA picker, real browser open. Next: Phase 5 (SSO / workforce identity, `/gcp-check` first) on user go.

## Domain Knowledge (GCP auth)

Things the tool works with; verify details against current Google docs rather than memory:

- **gcloud configurations**: named profiles stored under `~/.config/gcloud/`, each binding an account, project, and other properties. `gcloud config configurations` manages them.
- **Application Default Credentials (ADC)**: `~/.config/gcloud/application_default_credentials.json`, plus the `GOOGLE_APPLICATION_CREDENTIALS` and `GOOGLE_CLOUD_PROJECT` env vars.
- **Service account impersonation**: short-lived tokens via the IAM Credentials API (`generateAccessToken`), requiring `roles/iam.serviceAccountTokenCreator` on the target. Prefer impersonation over exported key files, always.
- **Global-state switching (decided)**: hop updates gcloud's own configuration (active configuration, project property, impersonation property) so every terminal shares one context and prompt tooling reflects it. The per-shell env-var/eval pattern granted.dev uses was considered and rejected by the user.

## Project Rules

@rules/architecture.md
@rules/rust.md
@rules/comments.md
@rules/security.md
@rules/cli-ux.md
@rules/cross-platform.md
@rules/docs.md
@rules/gcloud-safety.md
@rules/whats-next.md

## Verification

Once the Cargo project exists, the standard check sequence is:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build
```
