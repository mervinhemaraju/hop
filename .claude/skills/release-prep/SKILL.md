---
name: release-prep
description: Prepare a hop release end-to-end (version bump, changelog, checks, cross-platform builds, Homebrew formula, Windows artifact) stopping before any git or publish operation. Invoke with /release-prep [major|minor|patch or target version].
---

# Release Prep: $ARGUMENTS

Prepare everything for a release; the actual tagging, pushing, and publishing is done by me, never by you.

## 1. Preflight

- `git status` must be clean and `git log` reviewed for what is being shipped (read-only)
- Full check sequence passes: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- `cargo audit` reports no unaddressed advisories

## 2. Version

- Propose the semver bump with a one-line justification per changed area (breaking? feature? fix?)
- Update `version` in `Cargo.toml` (and `Cargo.lock` via `cargo build`)

## 3. Changelog

- Draft `CHANGELOG.md` entries from `git log` since the last release tag, grouped as Added / Changed / Fixed / Security
- Written for users of the CLI, not for developers reading the diff

## 4. Build Artifacts

Build release binaries for the target matrix in `rules/cross-platform.md`. If release tooling (e.g. cargo-dist) is already configured, use it; if proposing new tooling, verify its current docs first and treat it as a dependency decision (approval required).

- macOS and Linux: `.tar.gz` per target
- Windows: `.zip` containing `hop.exe`
- Generate `sha256` checksums for every artifact

## 5. Distribution

- **Homebrew**: draft the updated formula (version, per-target URLs, sha256 values) for the tap repository. Show me the formula diff; do not push anything to the tap.
- **Windows**: prepare the zip artifact and, if a winget/Scoop manifest exists in the repo, update it the same way (draft only).
- Update README install instructions if versions or install commands changed (per `rules/docs.md`)

## 6. Handoff

Summarise, then stop:

- Files changed (version, changelog, manifests, formula draft)
- Artifacts built, with paths and checksums
- The exact commands I need to run myself: commit, tag, push, create the GitHub release, upload artifacts, push the tap

Do NOT commit, tag, push, or publish anything. Do NOT create GitHub releases.
