//! The client side of an MCP plugin: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] calls the plugin
//! once it runs, or stops it. [`channel_response`] answers the channel
//! the provider opens — which, for a caller-served image, is the whole
//! of the image transfer.

//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] starts the plugin and hands back a [`Plugin`], which is
//! the scope and a way to wait for it to end. It takes THREE proxies —
//! an [`OciProxy`](crate::client::oci_proxy::OciProxy), a
//! [`PostgresProxy`](crate::client::postgres_proxy::PostgresProxy) and
//! a [`CommandProxy`](crate::client::command_proxy::CommandProxy) — and
//! puts one task on answering whichever the provider asks for.
//!
//! An [`agentic loop`](crate::endpoints::agentic_loop::run::client) is
//! asked for one thing while it runs; a plugin is asked for three, and
//! they arrive interleaved on one scope. That task is most of what is
//! in [`execute`].
//!
//! Dropping the [`Plugin`] sends the stop, which makes dropping the
//! ordinary way to be done with a plugin rather than a way to abandon
//! one.
//!
//! What is NOT here is calling the plugin. A caller sends
//! [`channel_request::Frame::Mcp`] through the
//! [`Handle`](crate::client::handle::Handle) it already has, quoting
//! [`Plugin::scope`].
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

#[cfg(feature = "client")]
mod execute;
#[cfg(feature = "client")]
mod plugin;

#[cfg(feature = "client")]
pub use execute::*;
#[cfg(feature = "client")]
pub use plugin::*;
