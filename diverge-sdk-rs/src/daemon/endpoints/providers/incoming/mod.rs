//! Incoming providers: the credentials of the ones that dial the
//! daemon.
//!
//! A provider that dials the daemon presents an unbrokered credential,
//! and the daemon judges it by the CREDENTIALS the client has added, as
//! a provider judges its own peers: each a key the daemon minted,
//! naming the identity a peer presenting it has, from one address if
//! one is given. The key that equals the presented one, byte for byte,
//! decides; none is a closed connection, and nothing of why reaches the
//! peer. The identity a credential names is the provider's, as
//! [`Identity::IncomingUnbrokered`](crate::daemon::endpoints::agents::logs::server::response::Identity::IncomingUnbrokered)
//! carries it.
//!
//! A credential is named by its identity, and nothing else: one
//! credential per identity. [`add`] makes one, and answers the key;
//! [`get`] answers one as a list would; [`list`] lists them, narrowed,
//! with their tags; [`delete`] takes one out, unless a provider is
//! connected through it; [`edit`] replaces one with another, answering
//! a new key, which is how a key rotates and how an address or an
//! identity changes; [`tag`] and [`untag`] change its tags, which are
//! the caller's. What a
//! list and a get report is an
//! [`Incoming`](list::server::response::Incoming): the credential,
//! which carries no key, with who is connected through it.
//!
//! # One connection per credential
//!
//! A provider connected through a credential holds it, and holds the
//! identity it names, for the connection's life. A second connection
//! that presents the same credential, or that is admitted as the same
//! identity, is closed without a word — as a credential the daemon
//! does not hold is — until the first connection has ended. So a
//! credential is connected through by exactly one provider or by none,
//! and `connected` in a list or a get says which.
//!
//! # The key is answered once
//!
//! A client never chooses a key and is never told one twice. The daemon
//! mints the key when a credential is added or replaced, answers it in
//! that response, and reports it nowhere after; a client that loses it
//! replaces the credential and receives another.

mod credential;

pub use credential::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
