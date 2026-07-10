# Cross-Platform Rules

hop ships to macOS/Linux (Homebrew) and Windows. Every change must work on all three; platform bugs are cheaper to prevent than to port away later.

## Target Matrix

- `aarch64-apple-darwin`, `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu` (musl worth considering for portability; decide when packaging)
- `x86_64-pc-windows-msvc`

## Paths & Files

- Never hardcode Unix paths. The gcloud config directory differs per OS: `~/.config/gcloud` on Linux, `~/.config/gcloud` (or the macOS equivalent gcloud actually uses; verify, don't assume) on macOS, `%APPDATA%\gcloud` on Windows. Resolve it in one adapter function, used everywhere.
- Build paths with `PathBuf`/`Path::join`, never string concatenation with `/`
- Home and config directories via a directories-style crate, not `$HOME` string reads
- `0600` permissions are Unix-only: `cfg`-gate them (`#[cfg(unix)]`) and use the closest Windows equivalent; never let the Windows build silently skip protecting sensitive files without a comment explaining the chosen approach

## Platform-Specific Code

- All `#[cfg(...)]` code lives in `adapters/`; `core/` stays platform-agnostic
- No Unix-only crates or APIs without a `cfg` gate and a Windows counterpart
- Do not shell out to Unix tools (`sh`, `sed`, `which`) at runtime

## Shell Integration

- Unix: bash, zsh, fish (export statements / eval pattern differ per shell)
- Windows: PowerShell (`$env:VAR = "..."` syntax, profile integration instead of rc files)
- The shell-output adapter is selected per shell; adding a shell must not touch `core/`

## CI

- Build and test on macOS, Linux, and Windows in CI; a change is not done if any leg is red
- Tests must not assume path separators, case-sensitive filesystems, or a POSIX shell
