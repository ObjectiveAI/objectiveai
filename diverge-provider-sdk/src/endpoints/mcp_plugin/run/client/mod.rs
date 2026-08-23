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
//! # What is not here, and is the next thing
//!
//! Calling the plugin. `channel_request::Frame::Mcp` is the frame for
//! it, and there is nothing that sends one: an `ExecuteHandle` gives
//! out no scope number, so the plugin cannot be reached from outside
//! this crate at all.
//!
//! Which means a plugin can be started, waited on, asked why it ended
//! and stopped — and not used. That is an honest intermediate state in
//! a crate where nothing is wired to anything yet, and it is not a
//! resting place: an `mcp` method on that type is now the only door in.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

// The executor is written and does not compile: it sends the tunneled
// MCP request that this endpoint no longer has, and reads an answer
// split into a head and a body. The five typed exchanges replaced both
// and nothing has been rewired to them yet.
//
// Left whole rather than gutted. What replaces it is a rewrite against
// the new shape, and this is the account of what the endpoint does.
//
// One line restores it.
// pub mod execute;
