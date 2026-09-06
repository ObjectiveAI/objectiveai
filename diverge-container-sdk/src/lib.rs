//! The SDK for a program inside a Diverge container.
//!
//! Every container the provider runs carries one proxy beside its
//! own entrypoint, on the container's loopback at port `14979`, and
//! that proxy is the container's whole way to the caller's world —
//! its tools, its database, its vault, the commands it may run. This
//! crate is that proxy as a client: the address hard-coded, the
//! features as methods, so no program inside a container names a
//! port or a path.
//!
//! Built one feature at a time, as the proxy is. Today: [`mcp`] —
//! the proxy's MCP server at `/mcp/agent`, which relays the four
//! exchanges to the caller's servers and delivers their
//! notifications.

pub mod mcp;

use diverge_provider_sdk::container_proxy;

/// The proxy's address for one of its paths, on the container's
/// loopback. Built from the provider SDK's port rather than written
/// out, so there is one copy of the number.
fn url(path: &str) -> String {
    format!("http://127.0.0.1:{}{}", container_proxy::PORT, path)
}
