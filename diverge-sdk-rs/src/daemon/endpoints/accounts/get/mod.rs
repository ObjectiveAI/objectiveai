//! Getting one account. One request, one answer. A client names an
//! account — by name, by a key credential's identity, by a hook
//! credential's resource; the daemon answers with the account as a list
//! would report it, without its key, that no account is the one named,
//! forbidden, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
