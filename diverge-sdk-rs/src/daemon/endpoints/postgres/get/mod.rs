//! Reading which database the daemon serves. One request, one answer. A
//! client asks; the daemon answers with its
//! [`Mode`](crate::daemon::endpoints::postgres::Mode) — local, or
//! remote with the URL, its password taken out — as its configuration
//! gave it, or that it failed, and the scope finishes. There is exactly
//! one database, so the request names nothing.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
