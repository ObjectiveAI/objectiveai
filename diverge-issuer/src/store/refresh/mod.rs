//! The refresh tokens, in chains.
//!
//! A chain is one sign-in: the client, the identity, the scope and the
//! audience granted, and when the identity signed in. Its first token
//! is issued with the sign-in's tokens — see [`issue`](Refresh::issue)
//! — and every refresh spends one token of it for the next — see
//! [`rotate`](Refresh::rotate). The rules:
//!
//! 1. A token is kept as its SHA-256, and is good for the configured
//!    days from its own issue.
//! 2. A rotation spends the token presented and issues the next in one
//!    transaction, so a token is spent at most once.
//! 3. A token presented again after it was spent is a token two
//!    parties hold, one of whom stole it: the whole chain is revoked,
//!    the token issued in its place with it, and the rotation is
//!    refused.
//! 4. A token of a revoked chain, a token past its days, a token
//!    presented by another client than its chain's, and a token there
//!    is no such one of, are refused, and change nothing.
//! 5. A chain is revoked by the revocation endpoint, by its identity's
//!    password changing, and by rule 3. It is removed with its client
//!    or its identity.
//! 6. A token past its days is removed at the next chain's issue; a
//!    spent one is kept until then, for rule 3.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod chain;
mod issue;
mod peek;
mod refresh;
mod revoke;
mod rotate;

pub use chain::*;
pub use refresh::*;
pub use rotate::*;
