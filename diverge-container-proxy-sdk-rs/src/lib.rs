//! The SDK for a program inside a Diverge container.
//!
//! Every container the provider runs carries one proxy beside its
//! own entrypoint, on the container's loopback at port `80`, and
//! that proxy is the container's whole way to the caller's world —
//! its tools, its database, its vault, the commands it may run. This
//! crate is that proxy as one [`Client`]: the address hard-coded,
//! every feature a method, one constructor and nothing to connect or
//! close — each feature's session opens the first time it is used —
//! so no program inside a container names a port or a path.
//!
//! Built one feature at a time, as the proxy is. Today: MCP — the
//! four exchanges the proxy relays to the caller's servers, their
//! methods in `mcp.rs`, their notifications ignored, and
//! [`mcp_url`] for a program whose MCP client is not this crate's —
//! the vault, the
//! keys the caller holds, in `vault.rs` — commands the caller runs,
//! in `command.rs` — Postgres, the caller's database on the
//! container's loopback, whose methods in `postgres.rs` are the
//! address and the URL the program's driver dials.
//!
//! The program's own server is not here. Every container's program
//! is its own HTTP server, on the loopback at the port [`port()`]
//! names, and the proxy dials it, forwarding what the provider's
//! server asks: `/register` and `/schema` on either kind of
//! container — [`register`] is the one's body and its answer, the
//! tools the program depends on — and, for an agent
//! container, the loop's `/run`, `/enqueue` and `/dequeue` that
//! [`agent`] states; for a tool container, the MCP server at `/mcp`
//! that [`tool`] states. Each surface is stated there, once, and
//! nothing in this crate stands between the two.

pub mod agent;
mod client;
mod command;
mod error;
mod mcp;
mod port;
mod postgres;
pub mod register;
pub mod tool;
mod vault;

pub use client::*;
pub use error::*;
pub use mcp::*;
pub use port::*;
pub use postgres::*;

/// The port the proxy listens for the program on, inside the
/// container: its HTTP surface — the vault, commands, the MCP server
/// — on the loopback, never published. The proxy binds it; this
/// crate dials it; one copy of the number.
pub const INSIDE_PORT: u16 = 80;

/// The proxy's address for one of its paths, on the container's
/// loopback — [`INSIDE_PORT`], the one it listens for the program
/// on.
fn url(path: &str) -> String {
    format!("http://127.0.0.1:{INSIDE_PORT}{path}")
}
