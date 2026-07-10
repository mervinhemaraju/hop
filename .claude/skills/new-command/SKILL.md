---
name: new-command
description: Guided workflow for adding a new hop subcommand end-to-end through all layers (core, adapters, commands, cli) with tests. Invoke with /new-command [description of the subcommand].
---

# New Command: $ARGUMENTS

Follow these steps in order. Do not skip ahead. The layer definitions live in `.claude/rules/architecture.md`.

## 1. Explore

- Read the existing command implementations to follow the established pattern
- Identify which core traits and adapters already exist that this command can reuse

## 2. Design (core first)

Working inward-out, design before writing:

- Domain types and newtypes the command needs in `core/`
- The trait (port) describing what it needs from the outside world, if not already covered
- The error type additions (`thiserror` variants)
- The command's user surface: name, args, flags, interactive flow AND its non-interactive equivalent, exit codes, exactly what goes to stdout vs stderr

**Stop here. Show me the design and wait for my approval.**

## 3. Implement, one layer at a time

1. `core/`: types, trait, logic, with unit tests using plain fakes
2. `adapters/`: trait implementation, mapping external failures into domain errors
3. `commands/`: orchestration, adapter construction and injection, exit code mapping
4. `cli/`: clap structs and conversion into command input; no logic

## 4. Verify against the rules

Before calling it done, check the change against:

- `rules/architecture.md`: dependency direction, no I/O in core
- `rules/cli-ux.md`: stdout discipline, TTY detection, non-interactive path works
- `rules/security.md`: no token material in output or errors

Then run:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 5. Summarise

List every file created or modified and what it contains. Do NOT commit or stage anything.
