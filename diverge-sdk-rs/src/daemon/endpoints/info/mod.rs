//! Asking the daemon what it is: its version. One request, one answer.
//!
//! The daemon's counterpart of the provider's
//! [`info`](crate::provider::endpoints::info), less the capabilities: a
//! daemon holds out none, so the answer is the version alone.
//!
//! The rules, in order:
//!
//! 1. The version is the daemon's own, `MAJOR.MINOR.PATCH`, read from
//!    its manifest when it was built. The daemon, the provider and this
//!    SDK are versioned together, so it is also the revision of the
//!    protocol the daemon speaks.
//! 2. Served here, it needs no grant and reads nothing: every connection
//!    the daemon admitted is answered [`Info`](server::response::Frame::Info).
//! 3. With `relay`, the daemon serves it on another daemon as every
//!    relayed request is served; a relay the account may not use is
//!    [`Forbidden`](server::response::Frame::Forbidden), and one that
//!    fails is [`Error`](server::response::Frame::Error).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
