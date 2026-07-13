# Download and install

hop is a single static binary. Every install method below ends with `hop` on
your `PATH` and nothing else on your system.

## Requirements

- The [Google Cloud SDK](https://cloud.google.com/sdk/docs/install)
  (`gcloud`) installed and on your `PATH`. hop delegates all login flows to
  it and reads its configuration files; without it, hop has nothing to
  switch.
- macOS (Apple Silicon or Intel), Linux (x86_64), or Windows (x86_64).

## Homebrew (macOS and Linux, recommended)

```sh
brew install mervinhemaraju/tap/hop
```

Homebrew adds the `mervinhemaraju/tap` tap automatically, downloads the
prebuilt binary for your OS and architecture, and verifies its SHA-256
checksum. Nothing is compiled locally.

Upgrade and uninstall:

```sh
brew update && brew upgrade hop
brew uninstall hop
```

## Prebuilt binaries (all platforms)

Every [GitHub release](https://github.com/mervinhemaraju/hop/releases)
attaches archives per target plus a `SHA256SUMS` file:

| Platform              | Asset                                        |
|-----------------------|----------------------------------------------|
| macOS (Apple Silicon) | `hop-<tag>-aarch64-apple-darwin.tar.gz`      |
| macOS (Intel)         | `hop-<tag>-x86_64-apple-darwin.tar.gz`       |
| Linux (x86_64, static)| `hop-<tag>-x86_64-unknown-linux-musl.tar.gz` |
| Windows (x86_64)      | `hop-<tag>-x86_64-pc-windows-msvc.zip`       |

Download, verify, extract, and place the binary somewhere on your `PATH`:

```sh
curl -sLO https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -sLO https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf hop-v0.1.0-x86_64-unknown-linux-musl.tar.gz hop
sudo mv hop /usr/local/bin/
```

The Linux binary is fully static (musl); it runs on any distribution with
no library requirements.

On Windows, extract `hop.exe` from the zip and add its folder to `PATH`
(Settings > System > About > Advanced system settings > Environment
Variables).

## From source (all platforms)

Requires Rust 1.87 or newer.

```sh
git clone https://github.com/mervinhemaraju/hop
cd hop
cargo install --path .
```

This builds and installs to `~/.cargo/bin/hop`. If you later switch to the
Homebrew package, remove the cargo copy first so the right one wins on
`PATH`:

```sh
cargo uninstall hop
hash -r
which hop    # should now be the brew path, e.g. /opt/homebrew/bin/hop
```

## Verify the installation

```sh
hop --version
hop status
```

`hop status` reads your local gcloud state and needs no network; if it
prints your active configuration, everything works. If it reports
`no gcloud configurations found`, gcloud has never been initialised; see
[Configuration](configuration.md).

## Shell setup

None. hop switches context by writing gcloud's own global state, so there
is nothing to source, no shell hooks, and no per-shell setup.
