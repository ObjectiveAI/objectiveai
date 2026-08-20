//! The client side of the agentic loop: what a client sends.
//!
//! [`request`] opens the scope. [`channel_response`] is what it sends
//! back on the channels the server opens inside that scope.
//!
//! There is no `response` here and no `channel_request`. A client does
//! not answer its own request, and it opens no channels of its own in
//! a loop — the chunks come back from the server, and the tunnels are
//! the server's to ask for.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] starts the loop and hands back an [`ExecuteStream`](execute::ExecuteStream) of
//! chunks. It is the first of these that does two things at once: a
//! loop is asked back while it is being listened to, so `execute` also
//! takes an [`McpProxy`](crate::client::mcp_proxy::McpProxy) and puts a
//! task on answering the agent's tool calls beside the stream.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes.

pub mod channel_response;
pub mod request;

#[cfg(feature = "client")]
pub mod execute;
