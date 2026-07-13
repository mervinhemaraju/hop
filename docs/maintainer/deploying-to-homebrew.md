# Deploying to Homebrew

hop is distributed through a personal tap:
[`mervinhemaraju/homebrew-tap`](https://github.com/mervinhemaraju/homebrew-tap).
There is no Homebrew account and no review process; the tap is a plain
GitHub repo, and pushing to it *is* the deployment.

## How the tap works

`brew install mervinhemaraju/tap/hop` resolves to the
`mervinhemaraju/homebrew-tap` repo (the `homebrew-` prefix is the naming
convention), reads `Formula/hop.rb`, downloads the release archive matching
the user's OS and architecture, verifies its SHA-256, and links `hop` into
the PATH. The formula serves prebuilt binaries; nothing compiles on user
machines.

Two copies of the formula exist:

- `packaging/homebrew/hop.rb` in the hop repo: the canonical, reviewed copy
- `Formula/hop.rb` in the tap repo: the deployed copy users install from

## Updating the formula for a new release (normal path)

Prerequisite: the GitHub release is **published** (draft assets have no
public URLs).

1. Open the tap repo's **Actions** tab
2. Select the **update-formula** workflow > **Run workflow**
3. Enter the release tag (e.g. `v0.2.0`) and run it

The workflow fetches the release's `SHA256SUMS`, rewrites the formula's
`version`, URLs, and per-target `sha256` values
(`.github/scripts/update_formula.py`), and pushes the commit as
`github-actions[bot]`. It refuses malformed tags, fails cleanly if the
release is missing or unpublished, and no-ops if the formula is already
current.

Afterwards, mirror the change into the canonical copy in the hop repo
(run the same script locally or copy the deployed file back).

## Updating manually (fallback)

```sh
curl -sL https://github.com/mervinhemaraju/hop/releases/download/vX.Y.Z/SHA256SUMS
```

Edit `Formula/hop.rb` in the tap: bump `version`, update the tag in every
`url`, and replace each `sha256` with the matching hash (three targets:
arm64 macOS, Intel macOS, musl Linux; the Windows zip is not served by
brew). Then:

```sh
git add Formula/hop.rb
git commit -m "hop vX.Y.Z"
git push origin main
```

## Verifying a deployment

```sh
brew update
brew install mervinhemaraju/tap/hop   # or: brew upgrade hop
hop --version                          # must print the new version
brew test hop
brew audit --strict hop
```

The gold-standard check for a first-time setup is a machine (or macOS user
account) that has never seen hop: `brew install mervinhemaraju/tap/hop`
followed by `hop status` must work out of the box.

## Adding a new formula to the tap

The tap holds all formulas for this account. For a new tool: add
`Formula/<name>.rb`, list it in the tap's README table, and (if it should
be auto-updated) extend the update workflow the same way as hop's.

## Formula maintenance rules

- The formula class name (`Hop`) must match the filename (`hop.rb`)
- Archives must contain the binary at their root; `bin.install "hop"`
  depends on that layout, which the release workflow guarantees
- The `sha256` is of the `.tar.gz`, not of the binary inside
- `version`, git tag, and `Cargo.toml` must always agree; the release
  workflow enforces the tag half, the update script the formula half
