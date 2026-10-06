//! Getting one volume. One request, one answer. A client names a
//! volume; the daemon answers with it as a list would report it, that
//! no volume is the one named, forbidden, or that it failed, and the
//! scope finishes. What a listing does not say — how much of the volume
//! is used, the hash of its content — a [`stat`](super::stat) does.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
