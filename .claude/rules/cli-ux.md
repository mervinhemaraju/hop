# CLI / UX Rules

hop is designed to be eval'd or sourced by a shell (like granted.dev). Output discipline is a correctness issue, not cosmetics.

## Output Streams

- **stdout is reserved for machine-consumable output**: export statements, completion scripts, JSON. Nothing else, ever.
- All human-facing messages (prompts, progress, warnings, errors) go to **stderr**
- Interactive prompts must render on stderr / the TTY so `eval "$(hop ...)"` still works

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
