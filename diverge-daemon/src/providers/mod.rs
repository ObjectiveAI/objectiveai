//! The connections to providers, in both directions, with the daemon
//! as the caller on each.
//!
//! A provider is the configuration's: an identity its
//! `providers.accepted` lists. There is no incoming and no outgoing —
//! a provider is served the same whichever end dialled. One the
//! configuration's `providers.connect` names is dialled: [`dial`] is
//! one task for the daemon's life, connecting to its address, making
//! the [`handshake`](diverge_sdk::wire::handshake) as the connector
//! from the entry's issuer, and dialling again five seconds after the
//! connection ends or the dial fails — as the provider's own dial
//! does; [`dial_all`] starts one per entry at start. One that dials
//! the daemon is admitted at the handshake by its identity, in
//! [`serve::connection`](crate::serve::connection). On either,
//! [`attach`] is the one shape: the socket split into the SDK's
//! client router and handle, the provider asked its info once — so
//! that a socket that is not a provider never counts as one
//! connected — the answer judged, and the handle put into
//! [`Live`](crate::daemon::Live) under the provider's identity with
//! the answer cached beside it: as CONNECTED, when the version
//! answered is the daemon's own, the one standing in which the
//! provider is used, and never for anything its cached capabilities
//! exclude; as a DIFFERENT VERSION otherwise, the socket held open
//! and the provider used for nothing. Then the router is driven until
//! the socket ends, and the slot given back. One connection per
//! identity: the slot is reserved before the info is asked, reading
//! as no connection until the answer fills it, and a newcomer that
//! collides is dropped without a word. On every connection used, when
//! the configuration says so, the daemon [`accept`]s other daemons
//! through the provider: one `daemons::accept` scope for the
//! connection's life, each connection the provider announces opened
//! by its own handshake and served by the [`DaemonAcceptor`] as the
//! account it admits. [`url`] is the one rule for a dialled address as
//! a URL. [`Error`] is why an attach did not happen.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod accept;
mod acceptor;
mod attach;
mod capacity;
mod dial;
mod error;
mod url;

pub use accept::*;
pub use acceptor::*;
pub use attach::*;
pub use capacity::*;
pub use dial::*;
pub use error::*;
pub use url::*;
