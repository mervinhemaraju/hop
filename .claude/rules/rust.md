# Rust Rules

## Toolchain & Style

- Latest stable Rust, 2021 edition or newer; pin `rust-version` in `Cargo.toml`
- `cargo fmt` and `cargo clippy -- -D warnings` must pass before any change is considered done
- Full type annotations on public APIs; document public items with `///` doc comments
- Prefer small modules with one responsibility; no `mod.rs` files (use `foo.rs` + `foo/` layout)

## Error Handling

- `thiserror` for library-style error types; `anyhow` only at the binary boundary
- Never `unwrap()` or `expect()` outside tests without a justifying comment
- Return `Result` from anything that can fail; fail fast with clear, user-facing error messages
- Error messages must tell the user what to do next, not just what broke

## Dependencies

- Dependencies are added deliberately: propose the crate and the reason before adding it
- Pin versions in `Cargo.toml`; commit `Cargo.lock` (this is a binary crate)
- Prefer well-maintained, widely-used crates; avoid abandoned or single-maintainer crates for anything security-sensitive

## Testing

- Unit tests live next to the code in `#[cfg(test)]` modules
- Integration tests live in `tests/`
- Arrange, act, assert structure in every test
- Mock external I/O (gcloud, network, filesystem); never let unit tests touch real GCP credentials
