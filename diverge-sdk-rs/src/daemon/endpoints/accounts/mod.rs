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
//! is how a client dials in as the account, judged as the provider
//! server judges inbound peers and as the daemon judges [incoming
//! providers](crate::daemon::endpoints::providers::incoming): a key the
//! client's credential must equal, byte for byte, from one address if
//! one is given, naming the identity the client then has; or a hook, a
//! directory [resource](crate::daemon::endpoints::resources) with
//! `hook.yaml` at its root, run as
//! [`providers`](crate::daemon::endpoints::providers) states, that
//! judges the credential and names the identity. Accounts with a
//! credential are tried in the order they were created, the first that
//! accepts deciding; none accepting is a closed connection, and nothing
//! of why reaches the peer. One account per name, one key account per
//! identity, one hook account per resource. An account with a name and
//! no credential is for containers alone; one with a credential and no
//! name dials in alone, and no container runs under it; one with both
//! does either.
//!
//! The identity a request is served under — what a
//! [`Creator::Client`](crate::daemon::creator::Client) carries — is the
//! account's name when it has one, and otherwise the identity its
//! credential names: a key account's `identity`, or the one its hook
//! answered.
//!
//! An account is named afterwards by a [`Reference`]: `{"name":…}`,
//! `{"identity":…}` for a key account, `{"authorize_hook":…}` for a
//! hook account; a named account with a key credential is reached
//! either way. [`create`] makes one, holding roles from the first;
//! [`get`] answers one as a list would; [`list`] lists them, narrowed;
//! [`delete`] removes one no container runs under and no client is
//! connected as; [`edit`] changes its name, its credential, its
//! description or its roles; [`tag`] and [`untag`] change its tags.
//! What a list and a get report is an
//! [`Account`](list::server::response::Account), its credential a
//! [`Told`], without the key.
//!
//! # Secrets are given, never answered
//!
//! A key is a credential. A list and a get report the account without
//! it; an edit replaces the credential whole, which is how one rotates.

mod credential;
mod definition;
mod reference;
mod told;

pub use credential::*;
pub use definition::*;
pub use reference::*;
pub use told::*;

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
