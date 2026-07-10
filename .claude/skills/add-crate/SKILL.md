---
name: add-crate
description: Vetting workflow before adding any dependency to hop. Checks maintenance, license, and security advisories, and justifies the crate against the stdlib. Invoke with /add-crate [crate name] [what it is needed for].
---

# Add Crate: $ARGUMENTS

hop handles credentials; every dependency is supply-chain surface. No crate is added without passing this workflow.

## 1. Justify

- State the problem the crate solves and which layer needs it (`cli`, `commands`, `core`, `adapters`)
- State why the stdlib or an already-present dependency is not enough
- `core/` dependencies get extra scrutiny: prefer keeping core close to dependency-free

## 2. Vet

Gather facts (crates.io, docs.rs, the repo); do not rely on memory for any of this:

- Latest version and release date; is it maintained?
- Download count and notable dependents
- License (must be compatible: MIT/Apache-2.0 family)
- Open RustSec advisories: run `cargo audit` after adding, or check https://rustsec.org/packages/ first
- Transitive dependency count; flag heavy trees

## 3. Propose

Present a short summary: crate, version to pin, license, maintenance verdict, tree weight, and the alternative considered.

**Stop here. Wait for my approval before touching Cargo.toml.**

## 4. Add & Verify

- Add with an exact or caret-pinned version in `Cargo.toml`
- Run `cargo audit` (install via `cargo install cargo-audit` if missing) and report the result
- Confirm `cargo build` and `cargo clippy -- -D warnings` still pass

Do NOT commit or stage anything.
