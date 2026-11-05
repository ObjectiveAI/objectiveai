//! Listing the host's identities. One request, one answer: every
//! identity signed in on the daemon's host that the filter lets through
//! and the grants reach, each with its tags, in order — or forbidden, or
//! a failure — and the scope finishes. The files are not watched, so the
//! answer is a snapshot.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
