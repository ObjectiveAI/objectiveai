//! Accounts: who a request is served for, and what they may do.
//!
//! An ACCOUNT is the one principal the daemon knows. Every request the
//! daemon serves is served for exactly one account — the account the
//! connection it arrived on dialed in as, or the
//! [`account`](crate::daemon::create::Inner::account) a container was
//! created under when the request comes from an agent or a tool — and
//! the daemon judges the request by that account's
//! [roles](crate::daemon::endpoints::roles): each a list of
//! [grants](crate::daemon::grant), the union of which is what the
//! account may do. A container created under no account holds no grant,
//! and everything it asks is answered `Forbidden`.
//!
//! # A name, a credential, or both
//!
//! An account is defined by a [`Definition`]: a `name`, a
//! [`Credential`], or both, and never neither. The name is what a
//! container's `account` names at its create, unique among accounts,
//! and what a request names the account by afterwards. The credential
//! is how a client dials in as the account: a key the daemon minted,
//! which the client presents byte for byte, from one address if one is
//! given, and the identity the client then has — judged as the provider
//! server judges inbound peers and as the daemon judges [incoming
//! providers](crate::daemon::endpoints::providers::incoming). The key
//! that equals the presented one decides; none is a closed connection,
//! and nothing of why reaches the peer. One account per name, one
//! credential per identity. An account with a name and no credential is
//! for containers alone; one with a credential and no name dials in
//! alone, and no container runs under it; one with both does either.
//!
//! The identity a request is served under — what a
//! [`Creator::Client`](crate::daemon::creator::Client) carries — is the
//! account's name when it has one, and otherwise the identity its
//! credential names.
//!
//! An account is named afterwards by a [`Reference`]: `{"name":…}`, or
//! `{"identity":…}` for one with a credential; a named account with a
//! credential is reached either way. [`create`] makes one, holding
//! roles from the first, and answers the key when a credential was
//! made; [`get`] answers one as a list would; [`list`] lists them,
//! narrowed; [`delete`] removes one no container runs under and no
//! client is connected as; [`edit`] changes its name, its credential,
//! its description or its roles, and answers a new key when it set a
//! credential; [`tag`] and [`untag`] change its tags. What a list and a
//! get report is an [`Account`](list::server::response::Account), its
//! credential without the key.
//!
//! # The key is answered once
//!
//! A client never chooses a key and is never told one twice. The daemon
//! mints the key when a credential is made — at the create, or at an
//! edit that sets one — answers it in that response, and reports it
//! nowhere after; an account that loses it has its credential set again
//! and receives another.

mod credential;
mod definition;
mod reference;

pub use credential::*;
pub use definition::*;
pub use reference::*;

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
