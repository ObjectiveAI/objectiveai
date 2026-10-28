//! Accounts: who a request is served for, and what they may do.
//!
//! An ACCOUNT is an identity — the `(issuer, subject)` a client's token
//! establishes in the mutual handshake — that this daemon admits, with
//! the roles it holds. Adding one is saying both: this identity may
//! connect to this daemon, and these are its permissions. A connection
//! whose token names an identity that is no account's, and no
//! provider's this daemon accepts, is closed in the handshake, with no
//! frame; a daemon with no account admits no client at all.
//!
//! Every request a client sends is served for exactly one account —
//! the account the connection it arrived on is — and the daemon judges
//! the request by that account's
//! [roles](crate::daemon::endpoints::roles): each a list of
//! [grants](crate::daemon::grant), the union of which is what the
//! account may do. No container is served for an account. A tool's
//! container, standalone or dependency, holds the grants its program
//! declared at its registration and nothing else — see the
//! `permissions` of the
//! [`register`](crate::container_proxy::inside::register) answer — and
//! an agent container asks the daemon nothing.
//!
//! An account is named by its identity, a [`Reference`]: one account
//! per identity. [`create`] makes one, holding roles from the first;
//! [`get`] answers one as a list would; [`list`] lists them, narrowed;
//! [`delete`] removes one no client is connected as; [`edit`] changes its description or its roles;
//! [`tag`] and [`untag`] change its tags. No account holds a key, and
//! no answer carries one: an identity's tokens are its issuer's.

mod reference;

pub use reference::*;

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
