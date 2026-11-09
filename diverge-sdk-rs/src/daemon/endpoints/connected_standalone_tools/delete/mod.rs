//! Deleting a connected tool.
//!
//! One request, one answer. A client names a connected tool of its
//! own; the daemon answers that the tool is deleted, that no connected
//! tool is the one named, that the tool is attached to an agent and was
//! left as it is, or that it failed, and the scope finishes. A deleted
//! tool's name is free for a [`create`](super::create). A tool attached
//! nowhere is used by nobody, so its connect is held by nothing, and a
//! delete lets nothing go. The tool on the other daemon is that
//! daemon's, and a delete here leaves it as it is.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
