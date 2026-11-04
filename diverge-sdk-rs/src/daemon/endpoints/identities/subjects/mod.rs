//! Listing the subjects signed in at one issuer. One request, one
//! answer: every subject an identity of the host's is signed in as at
//! the issuer named, in order, or forbidden, or a failure, and the
//! scope finishes. An issuer no identity is signed in at is answered
//! with none, not refused.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
