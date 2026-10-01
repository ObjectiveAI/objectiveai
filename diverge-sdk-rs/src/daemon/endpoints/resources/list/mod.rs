//! Listing a caller's resources.
//!
//! A client asks for its resources and the daemon sends every one it
//! holds under the caller's identity, one response each, oldest
//! uploaded first, then finishes: each by its id, its kind, when it
//! was uploaded, and how many bytes it holds. A caller with no
//! resources sees the finish and nothing before it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
