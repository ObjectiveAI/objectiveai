//! Listing a caller's agents.
//!
//! A client asks for its agents and the daemon sends every one it
//! holds under the caller's identity, one response each, oldest
//! created first, then finishes: what each is called, what image it
//! runs, whether it is active now, when its activity last changed
//! and where it ran, and how long its log is. A caller with no
//! agents sees the finish and nothing before it. The
//! daemon does not stay open; a caller that wants to know when an
//! agent's activity changes reads its [`logs`](super::logs).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
