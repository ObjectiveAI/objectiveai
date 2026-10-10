//! Forgetting a daemon. One request, one answer. A client names a
//! daemon of its own by name; the daemon answers that the daemon is
//! forgotten, that no daemon of the caller's has that name, that a
//! connected tool of the caller's names it and it was left as it is, or
//! that it failed, and the scope finishes. A forgotten daemon is
//! connected to no more, and its name is free for an add.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
