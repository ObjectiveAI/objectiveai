//! The connections to providers, in both directions, with the daemon
//! as the caller on each.
//!
//! An OUTGOING provider is dialled: [`dial`] is one task for the
//! provider's life on record, connecting to its address, presenting
//! the mode's authorization as the connection's first frame, and
//! dialling again five seconds after the connection ends or the dial
//! fails — as the provider's own dial does — until the task is ended
//! by the provider's delete or the daemon's stop; [`dial_all`] starts
//! one per record at start. An INCOMING provider dialled the daemon
//! and was admitted at the handshake by a credential the daemon
//! minted; [`incoming`] is what the socket becomes from then on. On
//! either, [`attach`] is the one shape: the socket split into the
//! SDK's client router and handle, the provider asked its version
//! once — so that a socket that is not a provider never counts as one
//! connected — the handle put into [`Live`](crate::daemon::Live)
//! under the provider's [`Identity`](diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity),
//! the router driven until the socket ends or the daemon evicts it,
//! and the slot given back. One connection per identity, and one per
//! incoming credential: the slot is taken before the version is asked,
//! and a newcomer that collides on either is dropped without a word.
//! On every connection held, when the configuration says so, the
//! daemon [`accept`]s other daemons through the provider: one
//! `daemons::accept` scope for the connection's life, each connection
//! the provider announces judged by its credential and served by the
//! [`DaemonAcceptor`] as the client it admits. [`url`] is the one
//! rule for the address of an outgoing provider as a URL. [`Error`]
//! is why an attach did not happen.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod accept;
mod acceptor;
mod attach;
mod error;
mod incoming;
mod outgoing;
mod refusing;
mod url;

pub use accept::*;
pub use acceptor::*;
pub use attach::*;
pub use error::*;
pub use incoming::*;
pub use outgoing::*;
pub use refusing::*;
pub use url::*;
