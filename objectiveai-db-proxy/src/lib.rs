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
//! There is NO configuration: the addresses and ports are hardcoded
//! ([`POSTGRES_PORT`], [`HOST_PORT`]), because a binary that gets
//! `podman exec`'d into an image somebody else built should not be
//! reconfigurable by that image's environment. [`run`] therefore takes
//! nothing, and the binary at `main.rs` only sets up logging and calls
//! it.

mod conduit;
mod frame;
mod run;

pub use conduit::*;
pub use frame::*;
pub use run::*;
