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
//! Every request the daemon serves is served for exactly one account —
//! the account the connection it arrived on is, or the
//! [`account`](crate::daemon::endpoints::standalone_tools::create::client::request::Frame::account)
//! a tool container was created under when the request comes from one
//! — and the daemon judges the request by that account's
//! [roles](crate::daemon::endpoints::roles): each a list of
//! [grants](crate::daemon::grant), the union of which is what the
//! account may do. A tool created under no account holds no grant, and
//! everything it asks is answered `Forbidden`; an agent container holds
//! no account and asks the daemon nothing. A dependency tool is the one
//! container served for no account: its template's grants are its
//! standing.
//!
//! An account is named by its identity, a [`Reference`]: one account
//! per identity. [`create`] makes one, holding roles from the first;
//! [`get`] answers one as a list would; [`list`] lists them, narrowed;
//! [`delete`] removes one no container runs under and no client is
//! connected as; [`edit`] changes its description or its roles;
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
