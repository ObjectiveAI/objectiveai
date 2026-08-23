//! The client side of a connection: what a connector sends.
//!
//! [`request`] opens the scope and asks to be let in.
//! [`channel_request`] reaches into the container once it is.
//! [`channel_response`] answers the one channel a provider opens
//! back — the content of a write.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] joins a laboratory and hands back TWO things, which no
//! other endpoint does: an [`ExecuteStream`](execute::ExecuteStream) of
//! the container's filesystem, and an
//! [`ExecuteHandle`](execute::ExecuteHandle) to reach into it with.
//!
//! They are split because a connection is two jobs at once and neither
//! is the other's subject. Every other endpoint has one — a
//! [`watch`](crate::endpoints::volumes::watch) only listens, a
//! [`plugin`](crate::endpoints::mcp_plugin::run) is almost silent — and
//! folding these two together would mean a connector that stopped
//! reading the filetree had stopped being able to read a file.
//!
//! Dropping the stream costs a view of the filesystem. Leaving is
//! [`disconnect`](execute::ExecuteHandle::disconnect), which is a
//! method rather than a destructor: a caller says when it is done, and
//! dropping the handle without saying so leaves the connection open as
//! far as the provider is concerned.
//!
//! It takes no proxies, unlike a plugin's, because there is nothing to
//! answer: a provider asks a connector for a write's content and for
//! nothing else on its own account.
//!
//! # Every ask
//!
//! [`ExecuteHandle`](execute::ExecuteHandle) has five against the
//! container's MCP server —
//! [`list_tools`](execute::ExecuteHandle::list_tools),
//! [`list_resources`](execute::ExecuteHandle::list_resources),
//! [`call_tool`](execute::ExecuteHandle::call_tool),
//! [`read_resource`](execute::ExecuteHandle::read_resource) and
//! [`notifications`](execute::ExecuteHandle::notifications) — and three
//! against the container itself:
//! [`read`](execute::ExecuteHandle::read),
//! [`write`](execute::ExecuteHandle::write) and
//! [`transfer`](execute::ExecuteHandle::transfer). The ninth thing a
//! connector can do is leave, and that is
//! [`disconnect`](execute::ExecuteHandle::disconnect).
//!
//! They are three shapes rather than one, because the exchanges are:
//!
//! | ask | goes out | comes back |
//! |-----|----------|------------|
//! | `transfer` | one request | one answer |
//! | the four MCP methods | one request | one answer, or the server's refusal |
//! | `read` | one request | a [`ReadStream`](execute::ReadStream) of the file |
//! | `notifications` | one request | an [`McpNotificationStream`](execute::McpNotificationStream), for as long as it is held |
//! | `write` | one request, and the content on a channel the PROVIDER opens | one answer |
//!
//! Every one of them can come back with the provider saying no. MCP was
//! once the exception, when the exchange was a tunnel and a refusal was
//! an HTTP status on a stream that was working — there is no status
//! now, so each of the five carries an error frame of its own.
//!
//! Only the write needed machinery. Its content cannot travel on the
//! channel that asked for it — only a responder can finish a channel —
//! so [`execute`] spawns a task that holds registered content until the
//! provider asks for it and routes by write id.
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
