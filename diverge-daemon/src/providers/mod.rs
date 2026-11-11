//! The connections to providers, in both directions, with the daemon
//! as the caller on each.
//!
//! A provider is a record: its identity, the direction its connection
//! is made in, whether it is enabled, and what the daemon holds open on
//! its connection — read by every connection from
//! the [`ProviderBook`]. Once connected, a provider is served the same
//! whichever end dialled. An enabled, outgoing one is dialled: [`dial`]
//! is one task while its record says so, connecting to its address,
//! making the [`handshake`](diverge_sdk::wire::handshake) as the
//! connector from the provider's own issuer, and dialling again five
//! seconds after the connection ends or the dial fails — as the
//! provider's own dial does; [`dial_all`] starts one per such record
//! at start, and [`reconcile`] is how a change to a record takes
//! effect on its connection and its dial. An enabled, incoming one that dials the daemon is admitted
//! at the handshake by its identity and its source address, in
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
//! collides is dropped without a word. On every connection used, the
//! daemon holds three scopes open as the provider's record says, each
//! independent of the other two — its [`Features`], made what the
//! record says by [`sync`] when the provider connects and after every
//! edit of the record, with the connection left as it stands: the
//! provider's `daemons::announce`, by which the daemon is in the
//! provider's list with the record's `announcement`; its
//! `daemons::accept`, when the record says `accept_daemons` — the
//! daemon [`accept`]s other daemons through the provider, each
//! connection the provider announces opened by its own handshake and
//! served by the [`DaemonAcceptor`] as the account it admits; and its
//! `daemons::listen`, when the record says `messageable` — see
//! [`listening`](crate::listening). A connection is closed for its
//! record only when the record is disabled, or edited so that its `ips`
//! no longer admit the connection's [`Source`]. [`url`] is the one rule
//! for a dialled address as a URL. [`Error`] is why an attach did not
//! happen.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod accept;
mod acceptor;
mod attach;
mod book;
mod capacity;
mod dial;
mod error;
mod features;
mod url;

pub use accept::*;
pub use acceptor::*;
pub use attach::*;
pub use book::*;
pub use capacity::*;
pub use dial::*;
pub use error::*;
pub use features::*;
pub use url::*;
