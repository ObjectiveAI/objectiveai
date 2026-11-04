//! Listing the issuers the host's identities are signed in at. One
//! request, one answer: every issuer at least one identity is signed in
//! at, in order, or forbidden, or a failure, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
