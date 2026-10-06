//! Changing an account. One request, one answer. A client names an
//! account and what is to change — its name, its credential, its
//! description, its roles, each as it is to be, taken away, or left as
//! it is; the daemon answers that the account is as the request states,
//! with a new key when the request set a credential, that no account is
//! the one named, that a role named is none the daemon has, that the
//! new name or identity is another account's, that the change would
//! leave the account with neither a name nor a credential, forbidden,
//! or that it failed, and the scope finishes. The request is applied
//! whole or not at all. Naming a role takes the `grant` grant over it.
//! Setting a credential replaces it whole and mints a new key, answered
//! once: how a key rotates, and how an address or an identity changes;
//! a connection a client holds as the account is not dropped by it, and
//! the next client is judged by the new key. A container running under
//! the account runs under its new name.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
