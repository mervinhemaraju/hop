---
name: rust-review
description: hop-specific review of current changes covering layering violations, error handling, stdout discipline, and credential safety. Invoke with /rust-review after implementing a feature or fix.
---

# Rust Review (hop)

Review the current changes (`git diff`, `git status`). This goes beyond the generic review; check each section in order.

## Layering (rules/architecture.md)

- Does any `core/` module import from `cli/`, `commands/`, or `adapters/`?
- Does `core/` perform I/O (filesystem, network, process spawn, env vars)?
- Is there logic hiding in `cli/` clap definitions?
- Are adapters constructed anywhere other than `commands/`?

## Error Handling

- Any `unwrap()`/`expect()` outside tests without a justifying comment?
- Are external failures mapped into typed domain errors at the adapter boundary, or is `anyhow` leaking inward?
- Do error messages tell the user what to do next?

## CLI Discipline (rules/cli-ux.md)

- Anything printed to stdout that is not machine-consumable output?
- Do new interactive flows have a non-interactive equivalent and TTY detection?
- Are exit codes correct and distinct per failure class?

## Security (rules/security.md)

- Any token, credential, or `~/.config/gcloud` content in logs, errors, or debug output?
- Any file written without `0600` where it holds sensitive data?
- Any shell command built by string concatenation?

## Code Quality

- Comments follow `rules/comments.md`: present where logic is non-obvious, absent where they restate the code
- Bare `String`s where a domain newtype exists?
- Dead code, unused imports, leftover debug prints?
- New tests present, arrange/act/assert, no real credentials touched?

## Checks

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Report

- ✅ What looks good
- ⚠️ Concerns (file + line)
- 🚫 Blockers (must fix before shipping)

Do NOT commit, stage, or fix anything as part of the review. Report only.
