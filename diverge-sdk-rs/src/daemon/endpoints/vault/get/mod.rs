//! Getting one vault key. One request, one answer. A client names a
//! key; the daemon answers with the key and every value it holds,
//! each with its number and who holds it, that no key has the name,
//! forbidden, or that it failed, and the scope finishes. The one
//! request that answers a value.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
