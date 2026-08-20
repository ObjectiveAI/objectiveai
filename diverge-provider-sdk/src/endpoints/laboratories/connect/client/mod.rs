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
//! other endpoint does: an [`ExecuteStream`] of the container's
//! filesystem, and an [`ExecuteHandle`] to reach into it with.
//!
//! They are split because a connection is two jobs at once and neither
//! is the other's subject. Every other endpoint has one — a
//! [`watch`](crate::endpoints::volumes::watch) only listens, a
//! [`plugin`](crate::endpoints::mcp_plugin::run) is almost silent — and
//! folding these two together would mean a connector that stopped
//! reading the filetree had stopped being able to read a file.
//!
//! Dropping the stream costs a view of the filesystem. Dropping the
//! handle sends the disconnect, which makes dropping the ordinary way
//! to leave rather than a way to abandon a connection.
//!
//! It takes no proxies, unlike a plugin's, because there is nothing to
//! answer: a provider asks a connector for a write's content and for
//! nothing else on its own account.
//!
//! # Three of the four asks
//!
//! [`ExecuteHandle`] has [`read`](ExecuteHandle::read),
//! [`write`](ExecuteHandle::write) and
//! [`transfer`](ExecuteHandle::transfer).
//! [`Mcp`](channel_request::Frame::Mcp) is the one left, and it wants
//! nothing the handle does not already hold.
//!
//! They are three shapes rather than one, because the exchanges are:
//!
//! | ask | goes out | comes back |
//! |-----|----------|------------|
//! | [`transfer`](ExecuteHandle::transfer) | one request | one answer |
//! | [`read`](ExecuteHandle::read) | one request | a [`ReadStream`] of the file |
//! | [`write`](ExecuteHandle::write) | one request, and the content on a channel the PROVIDER opens | one answer |
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
mod execute;
#[cfg(feature = "client")]
mod execute_handle;
#[cfg(feature = "client")]
mod execute_stream;
#[cfg(feature = "client")]
mod read_stream;

#[cfg(feature = "client")]
pub use execute::*;
#[cfg(feature = "client")]
pub use execute_handle::*;
#[cfg(feature = "client")]
pub use execute_stream::*;
#[cfg(feature = "client")]
pub use read_stream::*;
