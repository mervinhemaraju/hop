# Creating a new release

The release pipeline is tag-driven: GitHub Actions builds every platform
binary and drafts the release; a human reviews and publishes it. Nothing is
built on a laptop and nothing publishes without a click.

## Prerequisites

- CI (`ci.yml`) green on `dev`
- A clean working tree
- `cargo-audit` installed (`cargo install cargo-audit --locked`)

## 1. Prepare the release

Bump the version and write the changelog:

- `Cargo.toml`: set `version` to the new semver (then `cargo build` to
  update `Cargo.lock`)
- `CHANGELOG.md`: add a section for the version, grouped Added / Changed /
  Fixed / Security, written for CLI users rather than for developers
  reading the diff

Run the full check sequence; all of it must pass:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
cargo audit
```

## 2. Ship it to main

```sh
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Prepared release vX.Y.Z"
git push origin dev
# wait for CI to go green

git checkout main
git pull origin main
git merge dev
git push origin main
```

## 3. Tag

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```

The tag push triggers `release.yml`, which:

1. Fails fast if the tag does not match `Cargo.toml`'s version
2. Builds four targets in parallel: `aarch64-apple-darwin`,
   `x86_64-apple-darwin`, `x86_64-unknown-linux-musl` (static), and
   `x86_64-pc-windows-msvc`
3. Packages tar.gz archives (zip for Windows) with the binary, LICENSE, and
   README at the archive root
4. Generates a `SHA256SUMS` file over all artifacts
5. Creates a **draft** GitHub release with everything attached

## 4. Publish

GitHub > Releases > the draft > review the generated notes and the five
assets > **Publish release**. Draft assets have no public URLs, so
downstream steps (Homebrew) only work after publishing.

Then update the tap; see [Deploying to Homebrew](deploying-to-homebrew.md).

## If a build leg fails

Fix the cause, push the fix through `dev` to `main`, then move the tag:

```sh
git tag -d vX.Y.Z
git push --delete origin vX.Y.Z
git tag vX.Y.Z
git push origin vX.Y.Z
```

Also delete the stale draft release in the GitHub UI if one was created;
drafts survive tag deletion.

Re-tagging is only acceptable while the release is unpublished. Once a
release is public, never move its tag; ship a new patch version instead.

## Notes

- The `.gitattributes` file strips `.claude/` and `.github/` from the
  auto-attached "Source code" archives on each release.
- The Windows zip is published as a release asset only; winget/Scoop
  packaging is a future milestone.
