//! Domain layer: types, ports, and errors. Pure by rule; no I/O, no
//! imports from cli, commands, or adapters (rules/architecture.md).

pub mod context;
pub mod error;
pub mod ports;
pub mod settings;
// Dead-code allowance: the as_str accessors have no production caller until
// console URL building and API calls (Phases 3-4). Remove it then.
#[allow(dead_code)]
pub mod types;
