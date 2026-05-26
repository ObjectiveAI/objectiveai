//! Endpoint subcommands.
//!
//! Each module implements one streaming endpoint. Per-endpoint logic
//! is limited to: parsing the body into the SDK request type, calling
//! the matching `create_*_streaming` SDK function, and (for endpoints
//! whose final chunk carries an output value) emitting that final
//! value. Everything else — pipe lifecycle, chunk emission, the MCP
//! conduit — lives in shared modules (`pipes`, `streaming`,
//! `conduit`).

pub mod functions_executions;
