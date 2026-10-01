//! Listing a caller's templates.
//!
//! A client asks for its templates and the daemon sends every one it
//! holds under the caller's identity, one response each, oldest made
//! first, then finishes: each by its id, when it was made, and the
//! template whole. A caller with no templates sees the finish and
//! nothing before it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
