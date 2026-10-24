//! Providers: the ones the daemon accepts, and the ones it dials.
//!
//! A provider is where containers run and volumes live. Which providers
//! the daemon works with is its configuration's, `daemon.yaml`, read at
//! its start: the identities it accepts — each an `(issuer, subject)` —
//! and the addresses it dials, each with the issuer it presents as. No
//! request adds, edits or removes a provider. Whichever end dialled,
//! the mutual handshake establishes the provider's
//! [`Identity`](crate::shared::Identity), and the daemon works with a
//! provider only when that identity is one it accepts — the same list
//! for a provider it dialled as for one that dialled it. The daemon
//! holds at most one connection per provider identity at a time.
//!
//! [`get`] answers one provider by its identity, and [`list`] lists
//! them: every accepted identity, the address it is dialled at when the
//! configuration dials it, and where the daemon stands with it.
//!
//! # Where the daemon stands with each
//!
//! A list or a get reports the daemon's [`Status`] with the provider:
//! no connection; one the daemon uses, with the version and the
//! capabilities the provider answered its `info` ask, cached for the
//! connection's life; or one the daemon holds open and never uses,
//! the provider's version not being the daemon's. The daemon asks a
//! provider nothing its capabilities exclude, and counts it connected
//! only once the answer is cached. Beside the status, an item carries
//! the provider's latest `capacity` answer, which the daemon polls for
//! and caches; a list kept open hears each new answer as the item
//! changed.
//!
//! # And the daemons beyond them
//!
//! [`daemons`] are the other daemons this daemon reaches through its
//! providers, also its configuration's.

mod status;

pub use status::*;

pub mod daemons;
pub mod get;
pub mod list;
