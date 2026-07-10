# Documentation Rules

## Sync

- `--help` text and README usage sections are updated in the same change as the behaviour they describe; never in a follow-up
- If a flag, default, or exit code changes, grep the README and help text for the old value before calling the change done
- The README quick-start must always work verbatim on a fresh machine

## Help Text Style

- One-line summaries in the imperative: "Switch the active GCP project", not "Switches..." or "This command..."
- Every subcommand's long help includes at least one realistic example (with obviously fake values per the security rules)
- Document the non-interactive form alongside the interactive one
- Mention exit codes where a script author would need them

## README

- Structure: what it is, install (per platform), shell setup, usage per command, how impersonation works, troubleshooting
- Keep install instructions per-platform accurate (Homebrew, Windows); update them as part of any release packaging change
