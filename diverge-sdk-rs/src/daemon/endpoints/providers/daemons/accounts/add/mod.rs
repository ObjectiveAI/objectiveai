//! Adding an account to a daemon record. One request, one answer. A
//! client names a daemon of its own, a name for the account and the
//! mode this daemon authenticates to the remote in as it; the daemon
//! answers that the account is added, that no daemon of the caller's
//! has the name, that the daemon has an account of that name already,
//! forbidden, or that it failed, and the scope finishes. From then on
//! a connected tool and a relayed request may name the daemon and the
//! account, and this daemon connects as it, through the record's
//! links, for whatever either needs of the remote.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
