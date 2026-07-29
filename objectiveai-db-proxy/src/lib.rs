//! A Postgres-over-WebSocket conduit for ObjectiveAI plugin containers.
//!
//! A container cannot reach the machine hosting it, so a plugin inside
//! one cannot dial the ObjectiveAI database. This binary is copied into
//! the container — the way `objectiveai-mcp-laboratory` already is —
//! and bridges the two legs that DO work: it serves ordinary Postgres
//! clients on a fixed loopback port, and accepts a WebSocket from the
//! laboratory host, which dials in and relays to the real database.
//!
//! Every Postgres connection rides that one socket, keyed by a small
//! numeric id ([`frame`]). The point of shipping it as a separate
//! binary is that a plugin — in any language, using the plugin
//! framework or not — needs to know none of this. It sees a Postgres
//! server on localhost.
//!
//! Other crates can `use objectiveai_db_proxy::{ConfigBuilder, run}`
//! and spawn the proxy in-process; the binary at `main.rs` is a thin
//! wrapper that reads [`Config`] from the environment and calls [`run`].

mod conduit;
mod frame;
mod run;

pub use conduit::*;
pub use frame::*;
pub use run::*;
