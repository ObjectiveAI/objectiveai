//! Creating an account. One request, one answer. A client gives an
//! account's definition — a name, a credential, or both — with what it
//! is for in words and the roles it holds; the daemon answers that the
//! account is created, that the name, the identity or the resource is
//! an account's already, that a role named is none the daemon has, that
//! the hook names no resource the caller holds or not one that is a
//! hook, forbidden, or that it failed, and the scope finishes. Creating
//! takes the `create` grant over accounts and the `grant` grant over
//! every role named. The account holds its roles from the first request
//! served for it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
