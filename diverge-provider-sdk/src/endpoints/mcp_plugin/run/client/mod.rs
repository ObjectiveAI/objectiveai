//! The client side of an MCP plugin: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] calls the plugin
//! once it runs, or stops it. [`channel_response`] answers the channel
//! the provider opens — which, for a caller-served image, is the whole
//! of the image transfer.

//! # And, behind the `client` feature, a way to use it
//!
//! `execute` starts the plugin and hands back an `ExecuteHandle`,
//! which waits for it to end and says why. It takes THREE proxies —
//! an [`OciProxy`](crate::client::oci_proxy::OciProxy), a
//! [`PostgresProxy`](crate::client::postgres_proxy::PostgresProxy) and
//! a [`CommandProxy`](crate::client::command_proxy::CommandProxy) — and
//! puts one task on answering whichever the provider asks for.
//!
//! An [`agentic loop`](crate::endpoints::agentic_loop::run::client) is
//! asked for one thing while it runs; a plugin is asked for three, and
//! they arrive interleaved on one scope. That task is most of what is
//! in `execute`.
//!
//! `stop` ends the run, and
//! `wait` sees it through. They are two
//! methods rather than a destructor because a destructor could do
//! neither in sequence: it sent the frame and gave up the receiver in
//! the same move, so stopping a plugin and watching it go was not
//! something a caller could write.
//!
//! # Calling the plugin
//!
//! Five methods on that handle, one per MCP exchange: listing tools,
//! listing resources, calling a tool, reading a resource, and hearing
//! what the plugin says on its own account. They are the only door in,
//! because an
//! [`ExecuteHandle`](execute::ExecuteHandle) gives out no scope number
//! and nothing else can open a channel on the run.
//!
//! Four of them ask and are answered once. The fifth hands back a
//! stream, because a notification is not an answer — see
//! [`shared::mcp`](crate::shared::mcp) for why those five and why the
//! last one is shaped differently.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

#[cfg(feature = "client")]
pub mod execute;
