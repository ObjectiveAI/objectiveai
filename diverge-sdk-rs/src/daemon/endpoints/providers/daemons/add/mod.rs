//! Naming a daemon to connect to. One request, one answer. A client
//! gives a name and links; the daemon answers that the daemon
//! is added, that the name is a daemon's already, that a link names a
//! provider the caller has no record of, or that it failed, and the
//! scope finishes. From then on accounts may be added to it, and a
//! connected tool or a relayed request may name the daemon and one of
//! them, which this daemon connects to it as, through its links.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
