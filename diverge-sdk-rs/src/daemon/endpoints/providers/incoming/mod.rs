//! Incoming providers: the credentials of the ones that dial the
//! daemon.
//!
//! A provider that dials the daemon presents an unbrokered credential,
//! and the daemon judges it by CREDENTIALS the client has added, tried
//! in the order they were added, the first that accepts deciding: a
//! [`Credential::Key`], a string the credential must equal, naming the
//! identity a peer presenting it has; or a [`Credential::Hook`], a
//! directory resource the daemon runs to judge the credential and name
//! the identity, as [`providers`](super) states. None accepting is a
//! closed connection, and nothing of why reaches the peer. The identity
//! a credential names is the provider's, as
//! [`Identity::IncomingUnbrokered`](crate::daemon::endpoints::agents::logs::server::response::Identity::IncomingUnbrokered)
//! carries it.
//!
//! A credential is named afterwards by a [`Reference`]: a key
//! credential by the identity it names, a hook credential by its
//! resource, `authorize_hook`, the two members the server itself tells
//! its credentials apart by. [`add`] appends one; [`get`] answers one
//! as a list would; [`list`] lists them in the order they are tried,
//! narrowed; [`delete`] takes one out, unless a provider is connected
//! through it; [`edit`] replaces one with another of its kind, keeping
//! its place in the order. What a list and a get report is a [`Told`]:
//! the credential without its key.

mod credential;
mod reference;
mod told;

pub use credential::*;
pub use reference::*;
pub use told::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
