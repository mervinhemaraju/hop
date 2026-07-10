# hop: Milestone Plan (Foundation to Homebrew Release)

## Context

hop is a greenfield Rust CLI for GCP context switching, inspired by granted.dev's interactive UX. The repo currently has a hello-world scaffold (Cargo.toml, src/main.rs), full Claude rules/skills, and no dependencies. The user wants: launch GCP auth (including SSO / workforce identity), arrow-key navigation of projects, switching, browser console sessions, service account impersonation, and distribution via Homebrew (Windows later).

**Backend decision: hybrid.**
- gcloud binary: delegated to only for authentication flows (`gcloud auth login`, including SSO/workforce variants)
- gcloud config files: read for configurations/accounts (instant, offline)
- Direct API calls: Cloud Resource Manager for project listing (cached), IAM Credentials `generateAccessToken` for impersonation
- Browser: build console URLs and open them; no API involved

**Switching model (user decision): GLOBAL state, not per-shell.**
`hop switch` updates gcloud's own active configuration so ALL terminals share the same context and prompt tooling (starship gcloud module, zsh configs) reflects it immediately. No eval/export pattern, no shell integration command. Consequences:
- hop mutates gcloud config state on explicit user command (switch/impersonate); this is the product's core action
- Impersonation sets the `auth/impersonate_service_account` property in the active configuration (exact semantics verified via `/gcp-check`)
- How to write: prefer direct parse-modify-write of gcloud's INI config files for instant switching (shelling to `gcloud config set` costs 1-2s per call and would make the picker feel broken). Format verified against the real machine via `/gcp-check`; defensive parsing with clear errors per `rules/gcloud-safety.md`. Fallback if the format proves risky: `gcloud config set` behind the same adapter trait.
- Rule updates required in the same milestone: `rules/cli-ux.md` (drop the eval-pattern premise; stdout discipline stays as good practice), `.claude/CLAUDE.md` domain-knowledge bullet on env-var switching (replace with the global-state model), `rules/gcloud-safety.md` (hop MAY write gcloud config properties as its core function; everything else stays forbidden)

## Command Surface (v0.1)

- `hop login [account]` - launch GCP authentication; supports plain Google accounts AND SSO / workforce identity federation (see Phase 5)
- `hop` / `hop switch` - interactive arrow-key picker: account, then project; updates active gcloud config
- `hop console [--project X]` - open GCP console in browser for the active (or given) context
- `hop impersonate [sa]` - interactive SA picker; sets impersonation on the active config; `--clear` to stop
- `hop status` - show active context (account, project, impersonation)

## Phases

### Phase 1: Skeleton and foundation
Layer structure per `.claude/rules/architecture.md`:
- `src/cli/` clap definitions only
- `src/commands/` orchestration + exit codes (composition root)
- `src/core/` domain types (`ProjectId`, `AccountEmail`, `ServiceAccount`, `Context`), ports (traits), typed errors (`thiserror`)
- `src/adapters/` gcloud config reader AND writer (cross-platform path resolution in ONE function per `rules/cross-platform.md`), HTTP client, browser opener, prompts
- Replace hello-world main with clap dispatch + `ExitCode` mapping

Dependencies (each through `/add-crate` before landing):
- `clap` (derive); `inquire` or `dialoguer` (arrow-key prompts, rendered on stderr; vet both, pick one); `ureq` preferred over `reqwest` initially (sync, small tree, fast startup); `serde`/`serde_json`; `thiserror`; `anyhow` (binary boundary only); `open`; an INI crate if warranted (or hand-rolled defensive parser, decided in Phase 2)
- Note from pre-vetting (2026-07-11): clap 4.6.1 and thiserror 2.0.18 both pass vetting (MIT/Apache-2.0, no RustSec advisories). Using `std::env::home_dir` (un-deprecated in Rust 1.87) avoids a home-dir crate entirely; requires bumping `rust-version` to 1.87.

### Phase 2: Local context read/write (no network)
- Adapter: parse gcloud configurations (`configurations/config_*`, `active_config`); verify real formats on this machine read-only first
- Config WRITE path: switch active configuration / set project property, with a backup-and-restore safety net on write failure
- `hop status` and the account/config half of the picker
- Unit tests with tempdir fixture config directories; write tests assert byte-level preservation of untouched keys
- Create README.md per `rules/docs.md`

### Phase 3: Auth + projects (network)
- **`/gcp-check` first**: token acquisition from user creds (ADC vs `gcloud auth print-access-token` fallback), Resource Manager list/search endpoints, required scopes
- `hop login` (exec gcloud), project listing with local cache (instant arrow keys; `--refresh` flag), full `hop switch`

### Phase 4: Console + impersonation
- **`/gcp-check` first**: console URL parameters (project, authuser), `generateAccessToken`, `auth/impersonate_service_account` property semantics (gcloud AND ADC behaviour), SA listing API
- `hop console`, `hop impersonate` (+ `--clear`)
- `/rust-review` pass at the end (token-handling code lands here)

### Phase 5: SSO / workforce identity federation
- **`/gcp-check` first**: workforce identity federation login flows (`gcloud auth login --login-config`, login-config file format, `auth/login_config_file` property), workforce pool console URLs (they differ from standard console URLs), how workforce accounts appear in gcloud's credential store, impersonation constraints for workforce identities
- Extend `hop login` to detect/support SSO: `hop login --sso [--login-config <file>]` or auto-detect from gcloud config
- Extend `hop console` to build the correct console URL for workforce-federated sessions
- Extend `hop status` to show the identity type (Google account vs workforce federation)
- Tests with fake login-config fixtures; real-flow verification against the user's IdP (user provides a test tenant)

### Phase 6: Homebrew release (v0.1.0 ships)
- CI first: GitHub Actions matrix (macOS arm64/x86_64, Linux; Windows leg compile-only for now per `rules/cross-platform.md`) running fmt/clippy/test on every push
- Run `/release-prep` end to end: version confirmation, changelog, release builds for the target matrix, sha256 checksums
- Homebrew tap: create `homebrew-hop` tap repo structure + formula (drafted by `/release-prep`; user creates the repo and pushes)
- README install section (brew install, plus from-source and Windows build instructions) per `rules/docs.md`
- User performs: git tag, GitHub release + artifact upload, tap push (hop rules forbid Claude doing any of these)

## Critical Files

- `src/main.rs` (replace placeholder); new: `src/cli.rs`+`src/cli/`, `src/commands.rs`+`src/commands/`, `src/core.rs`+`src/core/`, `src/adapters.rs`+`src/adapters/` (no mod.rs, per `rules/rust.md`)
- `Cargo.toml` (deps via /add-crate; rust-version bump to 1.87)
- `.claude/rules/cli-ux.md`, `.claude/rules/gcloud-safety.md`, `.claude/CLAUDE.md` (model updates listed above)
- `README.md` (Phase 2), `.github/workflows/ci.yml` (Phase 6), Homebrew formula (Phase 6)

## Verification

- Per phase: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Phase 2: `hop status` against real local gcloud config (read-only); switch tested against a COPY of the config dir via `CLOUDSDK_CONFIG` before ever touching the real one
- Phase 3: real `hop switch`, then `gcloud config get project` and starship prompt confirm the change is globally visible
- Phase 4: impersonate a test SA in a sandbox project (user provides); confirm token works and is never printed
- Phase 5: full SSO login + switch + console round-trip against the user's IdP test tenant
- Phase 6: `brew install` from the tap on a clean machine; `hop status` works out of the box

## Out of Scope (later milestones)

- Windows packaging (winget/Scoop) and Windows CI test leg (code follows cross-platform rules from day one; packaging comes after Homebrew)
- Per-shell override mode (could return later as an opt-in flag if ever wanted)
- Multi-account credential caching beyond gcloud's own
