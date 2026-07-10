# gcloud / GCP Safety Rules

These apply both to hop's behaviour and to you (Claude) while developing and testing it.

## Scope of the Tool

- hop changes **local context only**: active configuration, project, account, impersonation state
- hop must never create, modify, or delete cloud resources; it is a switcher, not a management tool
- Any IAM interaction is limited to minting short-lived tokens via impersonation

## While Developing

- You may run read-only gcloud commands to gather context: `gcloud config list`, `gcloud config configurations list`, `gcloud auth list`, `gcloud projects list`
- Never run gcloud commands that mutate state (`gcloud config set`, `gcloud auth login`, `gcloud auth revoke`, anything under `gcloud iam`) without asking me first
- Never read or dump the contents of credential files (`application_default_credentials.json`, `credentials.db`, `access_tokens.db`); file names and existence checks are fine

## Verify, Don't Recall

- gcloud config file formats and API behaviours change; verify against current Google docs or actual (sanitized) local files rather than memory
- When hop parses gcloud's files or output, treat the format as unstable: parse defensively and fail with a helpful message on unexpected input
