//! Watching the container's tree.
//!
//! The server names what to leave out and gets the tree, then keeps
//! getting the changes to it until it says stop. Split by who SENDS:
//! the request and the stop go in [`client`], the stream comes back
//! in [`server`].
//!
//! The stop is what makes this a scope with two things to send. A
//! read answers and finishes; a tree reports until somebody says stop,
//! and the server may open as many as it likes, each a subscription
//! of its own that starts whole.
//!
//! The stream itself is [`filetree`](crate::shared::filetree)'s, not a
//! second description of the same thing.

pub mod client;
pub mod server;
