# hop

## What This Is

`hop` is a CLI tool written in Rust for fast, interactive context switching on Google Cloud Platform; think [granted.dev](https://granted.dev), but for GCP instead of AWS.

Core capabilities (planned):

- Switch between GCP projects of already-authenticated accounts, interactively or by name
- Switch between multiple authenticated gcloud accounts
- Impersonate service accounts interactively (short-lived credentials, no key files)
- Keep the active context visible and easy to inspect

The architecture and command surface are not finalized yet. Do not invent commands, flags, or module layouts beyond what exists in the code; when the design is ambiguous, ask before assuming.

## Status

Cargo scaffold only: a hello-world `src/main.rs` and a zero-dependency `Cargo.toml` (edition 2024). No feature code exists yet.

The milestone plan is agreed and lives at `.claude/PLAN.md` (6 phases: skeleton, local context read/write, auth + projects, console + impersonation, SSO / workforce identity, Homebrew release). Key decisions already made:

- **Backend: hybrid.** gcloud binary for login flows only; gcloud config files read directly; direct API calls for project listing and impersonation; console via URL building.
- **Switching: global state.** hop writes gcloud's own active configuration so all terminals and prompt tooling (starship) reflect the change. No per-shell env-var/eval pattern.

Do not start a phase without the user explicitly saying to begin it. Plan approval alone is not the signal to write code.

**Working mode (user decision, 2026-07-11): step-by-step delivery.** Implement in small runnable increments, not whole phases in one go. After each increment: show what was built, give the user the exact commands to run it themselves, and wait for their approval before the next increment. Keep the "Current step" line below up to date.

Current step: Phase 1 increment 2 (real `hop status`: config dir resolution + active config name, core/adapters layers, thiserror) delivered, awaiting user approval. Increment 1 (clap surface + stubs) approved.

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

## Verification

Once the Cargo project exists, the standard check sequence is:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build
```
