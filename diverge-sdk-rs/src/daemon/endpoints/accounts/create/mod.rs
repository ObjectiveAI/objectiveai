//! Creating an account. One request, one answer. A client gives an
//! account's definition — a name, a credential without its key, or both
//! — with what it is for in words and the roles it holds; the daemon
//! answers that the account is created, with the key it minted when a
//! credential was given, that the name or the identity is an account's
//! already, that a role named is none the daemon has, forbidden, or
//! that it failed, and the scope finishes. Creating takes the `create`
//! grant over accounts and the `grant` grant over every role named. The
//! account holds its roles from the first request served for it. The
//! key is never reported again.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
