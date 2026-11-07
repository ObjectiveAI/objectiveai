//! Changing a remote daemon's record: its description, and nothing else.
//! What it reaches and as whom — the remote, our account there, the
//! routes — is its [`DaemonId`](super::DaemonId) and never changes;
//! reaching elsewhere is a delete and an add. A connection held to it is
//! kept.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
