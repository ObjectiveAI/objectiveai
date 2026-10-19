//! Forgetting an account on a daemon record. One request, one answer.
//! A client names a daemon of its own and an account of it; the daemon
//! answers that the account is forgotten, that no daemon of the
//! caller's has the name or the daemon no account of that name, that a
//! connected tool of the caller's names the account and it was left as
//! it is, forbidden, or that it failed, and the scope finishes. The
//! connection this daemon holds as the account, if any, is let go.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
