//! Domain layer: types, ports, and errors. Pure by rule; no I/O, no
//! imports from cli, commands, or adapters (rules/architecture.md).

pub mod console;
pub mod context;
pub mod error;
pub mod ports;
pub mod settings;
pub mod types;
pub mod workforce;
