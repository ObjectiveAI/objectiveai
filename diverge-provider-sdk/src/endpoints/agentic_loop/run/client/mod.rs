//! The client side of the agentic loop: what a client sends.
//!
//! [`request`] opens the scope. [`channel_response`] is what it sends
//! back on the channels the server opens inside that scope.
//! [`channel_request`] is the channels it opens itself: two about
//! the running conversation's queue — an
//! [`enqueue`](channel_request::Frame::Enqueue) puts a message in,
//! a [`dequeue`](channel_request::Frame::Dequeue) clears it — and
//! the caller's half of a database connection, a
//! [`postgres`](channel_request::Frame::Postgres).
//!
//! There is no `response` here. A client does not answer its own
//! request — the chunks come back from the server.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] starts the loop and hands back an [`ExecuteStream`](execute::ExecuteStream) of
//! chunks. It is the first of these that does two things at once: a
//! loop is asked back while it is being listened to, so `execute` also
//! takes an [`McpProxy`](crate::client::mcp_proxy::McpProxy), a
//! [`FetchProxy`](crate::client::fetch_proxy::FetchProxy) and a
//! [`PostgresProxy`](crate::client::postgres_proxy::PostgresProxy)
//! and puts a task on answering the server's asks — the agent's tool
//! calls, the provider's fetches, the container's database
//! connections — beside the stream.
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
