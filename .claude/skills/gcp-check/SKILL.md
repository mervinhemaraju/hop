---
name: gcp-check
description: Verify current GCP API and gcloud behaviour from official docs before implementing any auth-touching code. Invoke with /gcp-check [what is being implemented] before writing a plan for anything touching GCP APIs, OAuth, ADC, or gcloud file formats.
---

# GCP Check: $ARGUMENTS

hop's correctness depends on external surfaces Google controls and changes. Never implement against remembered API shapes. Follow these steps in order.

## 1. Identify the Surfaces

List exactly which external surfaces the change touches:

- IAM Credentials API (e.g. `generateAccessToken`, `generateIdToken`)
- OAuth 2.0 / token endpoints
- Application Default Credentials resolution order
- gcloud config files and databases under the gcloud config directory
- gcloud CLI commands and their output formats

## 2. Fetch Current Docs

For each surface, fetch the official documentation now:

- API references under `https://cloud.google.com/iam/docs/reference/credentials/rest`
- Auth concepts under `https://cloud.google.com/docs/authentication`
- gcloud CLI reference under `https://cloud.google.com/sdk/gcloud/reference`

Confirm from the fetched docs (not memory): endpoint URLs, request/response field names, required scopes and roles, token lifetimes and their limits, and any deprecation notices.

## 3. Verify Local Reality

Where the change parses local gcloud state, check the actual format on this machine using read-only commands per `rules/gcloud-safety.md` (file names, structure via sanitized samples; never dump credential contents).

## 4. Plan

Write a short implementation plan referencing only doc-confirmed endpoints, fields, and behaviours. Note anything where the docs contradicted expectations.

**Stop here. Show me the plan and wait for approval before implementing.**
