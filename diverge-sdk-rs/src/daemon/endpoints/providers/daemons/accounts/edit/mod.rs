//! Changing an account's mode on a daemon record: a patch that gives
//! the mode whole. One request, one answer. A client names a daemon of
//! its own, an account of it, and a patch — see
//! [`patch`](crate::daemon::patch) — to the mode; the daemon answers
//! that the account has it, that no daemon of the caller's has the
//! name or the daemon no account of that name, forbidden, or that it
//! failed, and the scope finishes. This is how a credential rotates:
//! the connection this daemon holds as that account, if any, is let
//! go, and the next one opened presents the new credential.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
