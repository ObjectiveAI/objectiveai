//! Deleting a vault key. One request, one answer. A client names a
//! key; the daemon answers that the key is gone, that no key has the
//! name, that an agent or a tool binds it and it was left as it is,
//! forbidden, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
