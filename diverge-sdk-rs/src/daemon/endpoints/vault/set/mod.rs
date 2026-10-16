//! Setting one vault key. One request, one answer. A client names a
//! key and gives its value; the daemon answers that the key holds it
//! — made where the vault had no such key, replaced where it had —
//! forbidden, or that it failed, and the scope finishes. A key some
//! agent or tool binds is set like any other: its next start is made
//! with the new value.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
