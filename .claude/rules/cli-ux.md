# CLI / UX Rules

hop switches context by writing gcloud's own global state; it is not eval'd or sourced by the shell (that granted.dev-style model was considered and rejected). Output discipline stays a hard rule regardless: pipes and scripts depend on it.

## Output Streams

- **stdout is reserved for machine-consumable output**: completion scripts, JSON, future scriptable output. Nothing else, ever.
- All human-facing messages (prompts, progress, warnings, errors) go to **stderr**
- Interactive prompts must render on stderr / the TTY so stdout stays clean even when piped

## Exit Codes

- `0` success, non-zero on failure
- Distinct exit codes for distinct failure classes (not authenticated, permission denied, user aborted, bad input)
- A user cancelling an interactive prompt (Ctrl+C / Esc) is not an error crash; exit cleanly with a dedicated code

## Interactivity

- Every interactive flow must have a non-interactive equivalent (flags or args) so hop is scriptable
- Detect TTY: if stdin/stderr is not a TTY, never block on a prompt; fail with a clear message telling the user which flag to pass
- Respect `NO_COLOR` and disable color when output is not a TTY

## Performance

- Startup must feel instant; this tool runs dozens of times a day
- No network calls unless the command actually needs one; listing local configurations must work offline
