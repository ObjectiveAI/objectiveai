//! Changing a daemon record. One request, one answer. A client names a
//! daemon of its own and gives new links; the daemon answers that the
//! record is changed, that no daemon of the caller's has the name, that
//! a link names a provider the caller has no record of, or that it
//! failed, and the scope finishes. A connection this daemon holds to it
//! now is not dropped; the next one opened goes through the new links.
//! An account's mode is changed by its own
//! [`accounts::edit`](super::accounts::edit).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
