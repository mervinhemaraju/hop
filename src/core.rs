//! Domain layer: types, ports, and errors. Pure by rule; no I/O, no
//! imports from cli, commands, or adapters (rules/architecture.md).

pub mod context;
pub mod error;
pub mod ports;
// Dead-code allowance: the identifier constructors and accessors have no
// caller until the Phase 2 configuration-file parser. Remove it then.
#[allow(dead_code)]
pub mod types;
