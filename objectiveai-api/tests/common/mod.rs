//! Shared infrastructure for `objectiveai-api` integration tests.
//!
//! Each integration test binary (`tests/<layer>.rs`) starts the
//! `objectiveai-api` binary as a child process and drives it via HTTP.
//! Each test binary owns its own server process — separate ephemeral
//! port pool, separate MCP proxy. Cargo runs the test binaries in
//! parallel, so layer-level isolation is automatic.
//!
//! Wiring conventions:
//!
//! - `tests/<layer>.rs` does `mod common;` to pull this directory in.
//! - `common::server::client()` returns a process-wide `Arc<HttpClient>`
//!   pointing at the spawned api server. First call boots the server.
//! - `common::stream_harness::*` is the same set of stream-consume
//!   helpers + `assert_snapshot` as the old `src/stream_harness.rs`.
//! - `common::mcp_server::spawn(name, tools)` boots a local rmcp test
//!   server in the test process; tests embed its URL into agent
//!   `mcp_servers` and the api server's proxy fans out to it.

#![allow(dead_code)]

pub mod mcp_server;
pub mod server;
pub mod stream_harness;
