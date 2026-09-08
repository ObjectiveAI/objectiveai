//! The SDK for a program inside a Diverge container.
//!
//! Every container the provider runs carries one proxy beside its
//! own entrypoint, on the container's loopback at port `14979`, and
//! that proxy is the container's whole way to the caller's world —
//! its tools, its database, its vault, the commands it may run. This
//! crate is that proxy as one [`Client`]: the address hard-coded,
//! every feature a method, one constructor and nothing to connect or
//! close — each feature's session opens the first time it is used —
//! so no program inside a container names a port or a path.
//!
//! Built one feature at a time, as the proxy is. Today: MCP — the
//! four exchanges the proxy relays to the caller's servers, their
//! methods in `mcp.rs`, their notifications ignored — the vault, the
//! keys the caller holds, in `vault.rs` — commands the caller runs,
//! in `command.rs` — Postgres, the caller's database on the
//! container's loopback, whose methods in `postgres.rs` are the
//! address and the URL the program's driver dials.
//!
//! The loop is not here. An agent container's program is its own
//! HTTP server, on the loopback at the port the provider SDK's
//! [`container_proxy::agent`] names — `/run`, `/schema`, `/enqueue`,
//! `/dequeue` — and the proxy dials it, forwarding what the
//! provider's server asks; that surface is stated there, once, and
//! nothing in this crate stands between the two.

mod client;
mod command;
mod error;
mod mcp;
mod postgres;
mod vault;

pub use client::*;
pub use error::*;
pub use postgres::*;

use diverge_provider_sdk::container_proxy;

/// The proxy's address for one of its paths, on the container's
/// loopback. Built from the provider SDK's port rather than written
/// out, so there is one copy of the number.
fn url(path: &str) -> String {
    format!("http://127.0.0.1:{}{}", container_proxy::PORT, path)
}
