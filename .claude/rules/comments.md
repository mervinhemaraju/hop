# Comment Rules

Comments exist so a reader can understand what a part of the code is doing without reverse-engineering it. Add them where they earn their place; never drown the code in them.

## When a comment is required

- Non-obvious logic: anything a competent Rust developer could not follow on a first read (bit tricks, subtle ordering requirements, workarounds)
- Domain behaviour: places where the code encodes GCP/gcloud behaviour that is not visible from the code itself (file formats, API quirks, auth flows)
- Section-level orientation in longer functions or modules: a one-line comment above a logical block ("resolve the active configuration", "mint the impersonated token") so the shape of the flow is scannable
- The *why* behind a decision that looks wrong or arbitrary but is deliberate
- Every justified `unwrap()`/`expect()` or `unsafe` block (per the Rust rules)

## When a comment is banned

- Restating what the line obviously does (`// increment counter`)
- Narrating the diff or the change history ("changed this to fix X"); that belongs in the commit message
- Commented-out code; delete it, git remembers
- Placeholder noise (`// TODO: improve this`) without a concrete, actionable note

## Style

- `///` doc comments on all public items (crate API surface); regular `//` for internal explanation
- Keep comments short and close to the code they describe; if a comment needs a paragraph, consider whether the code should be restructured or the explanation belongs in module-level docs (`//!`)
- Update or delete comments when the code they describe changes; a stale comment is worse than none
