//! Seeing what a resource holds: one snapshot of its tree.
//!
//! A client names a directory resource and a subtree of it, empty for
//! the whole; the daemon answers with the tree as it is, once — which,
//! a resource being its hash, is as it ever is — that no resource is
//! the one named or nothing is at the path, that what is there is a
//! file, forbidden, or that it failed, and the scope finishes. Nothing
//! is watched and no change is sent: a resource never changes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
