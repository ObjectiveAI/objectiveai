//! Deleting a vault key, or one of its values. One request, one
//! answer. A client names a key — and, when it means one value of
//! it, which; the daemon answers that the key or the value is gone,
//! that no key has the name or the key no such slot, that an agent
//! or a tool binds the key and it was left as it is, that a running
//! container holds the value — or, for a whole key, one of its
//! values — and it was left as it is, that the slot is the key's
//! last and the key is what to delete, forbidden, or that it failed,
//! and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
