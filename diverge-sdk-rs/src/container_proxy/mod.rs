//! The proxy beside every container's program, from both sides.
//!
//! Every container a provider runs carries one proxy beside its own
//! entrypoint, and the proxy has two faces. [`outside`] is the
//! WebSocket the provider's server opens into it — this crate's own
//! wire, with the provider as its client and the proxy as its server
//! — carrying the scopes that begin a container, mount a FUSE
//! filesystem, and read, write and watch the container's files.
//! [`inside`] is the loopback the container's program dials the
//! proxy on: one [`Client`](inside::Client) with every feature a
//! method — MCP, the vault, commands, Postgres — and the contract of
//! the program's own server the proxy dials back. Nothing crosses
//! from one face to the other but through the proxy itself.

pub mod inside;
pub mod outside;
