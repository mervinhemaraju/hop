# Architecture Rules

hop follows a layered architecture with strict boundaries. Every change is checked against these; violations are refactored before merging, not accumulated.

## Layers

```
cli/        clap definitions and argument parsing only; no logic
commands/   orchestration per subcommand; wires core to adapters, owns exit codes
core/       domain types and logic (contexts, accounts, impersonation); pure
adapters/   the outside world: gcloud files, IAM Credentials API, shell output, prompts
```

## Boundary Rules

- Dependencies point inward only: `cli -> commands -> core`, `adapters -> core`. Nothing in `core/` may import from `cli/`, `commands/`, or `adapters/`.
- `core/` performs no I/O: no filesystem, no network, no process spawning, no reading env vars. It defines traits for what it needs; `adapters/` implement them.
- `commands/` is the only place where core and adapters meet (composition root pattern). Construct adapters there and inject them.
- `cli/` contains only clap derive structs/enums and conversion into command inputs. If an `if` in `cli/` is making a decision, it belongs in `commands/` or `core/`.

## Design Defaults

- Traits for ports (what core needs from the world), structs for adapters (how it is provided); this keeps the "shell out to gcloud vs call APIs directly" decision swappable
- Constructor injection; no global state, no lazy statics for dependencies
- Domain errors are typed per module with `thiserror`; adapters map external failures into them at the boundary
- Newtypes for domain identifiers (`ProjectId`, `AccountEmail`, `ServiceAccount`) instead of passing bare `String`s around
- Make invalid states unrepresentable: prefer enums over boolean flags and option combinations

## Testability Check

A change respects this architecture if:

- `core/` logic is unit-testable with plain fakes, no filesystem or network mocking frameworks
- an adapter can be replaced (fake gcloud, in-memory config) without touching `core/` or `cli/`
