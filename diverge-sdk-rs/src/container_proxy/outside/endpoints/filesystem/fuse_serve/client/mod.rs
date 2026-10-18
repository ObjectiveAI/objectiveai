//! The client side of a serve: what the server sends.
//!
//! [`request`] opens it — the subtree, once. [`channel_request`] is
//! what the server opens on it: the nine asks, the stop that ends
//! the scope, and the tree.
//!
//! There is no `channel_response`. The proxy opens no channel on a
//! serve: it answers.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::wire::client::handle::Handle) to the proxy
//! and the path, and get back an
//! [`ExecuteHandle`](execute::ExecuteHandle) that asks one ask at a
//! time, relays one as it came, opens the tree, and stops the scope.

pub mod channel_request;
pub mod request;

pub mod execute;
