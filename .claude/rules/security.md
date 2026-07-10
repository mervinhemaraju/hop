# Security Rules (non-negotiable)

## Credentials

- Never write service account keys to disk; impersonation and short-lived tokens only
- Never log, print, or include in error messages any access token, refresh token, or credential material; redact in debug output too
- Any file hop writes that contains credentials or cached tokens must be created with `0600` permissions
- Treat everything under `~/.config/gcloud/` as sensitive; read what is needed, modify only with clear user intent

## Code

- Validate and sanitize all external input (project IDs, account emails, service account names) before using it in commands or API calls
- Never build shell command strings by concatenation; pass arguments as arrays
- No telemetry, no phoning home, no update checks that send data

## Development

- Never use real production credentials in tests or examples
- Sample configs and docs use obviously fake values (`my-project-123`, `sa@my-project-123.iam.gserviceaccount.com`)
