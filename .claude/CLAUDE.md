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

Greenfield. There is no Cargo project yet; the repo contains only this file, the rules, and a `.gitignore`. The first milestone (scaffolding, dependency choices, command structure) will be planned together before any code is written.

## Domain Knowledge (GCP auth)

Things the tool works with; verify details against current Google docs rather than memory:

- **gcloud configurations**: named profiles stored under `~/.config/gcloud/`, each binding an account, project, and other properties. `gcloud config configurations` manages them.
- **Application Default Credentials (ADC)**: `~/.config/gcloud/application_default_credentials.json`, plus the `GOOGLE_APPLICATION_CREDENTIALS` and `GOOGLE_CLOUD_PROJECT` env vars.
- **Service account impersonation**: short-lived tokens via the IAM Credentials API (`generateAccessToken`), requiring `roles/iam.serviceAccountTokenCreator` on the target. Prefer impersonation over exported key files, always.
- **Env-var based switching**: like granted.dev, changing the active context for a shell likely means printing export statements or integrating with the shell (subshell, shell function, or eval pattern); a child process cannot mutate the parent shell's environment.

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
