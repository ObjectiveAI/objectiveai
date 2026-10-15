//! Walking a volume: how much of it is used, and the hash of its
//! content. One request, one answer. A client names a volume; the
//! daemon asks the provider, which walks the volume once, at rest, and
//! answers the two numbers a listing leaves out because each costs the
//! walk, that no volume is the one named, that the volume is held,
//! forbidden, or that it failed, and the scope finishes. The rest of
//! what is known about the volume is a [`get`](super::get)'s.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
