//! Providers: the records of the providers the daemon works with.
//!
//! A provider is where containers run and volumes live. Which providers
//! the daemon works with is its records', kept in its database: one per
//! provider [`Identity`](crate::shared::Identity), each with the
//! [`Direction`] its connection is made in — the provider dials the
//! daemon, from the IPs it may, or the daemon dials it — whether
//! the daemon is discoverable to it, and whether it is enabled. The
//! rules:
//!
//! 1. **By identity.** Whichever end dialled, the mutual handshake
//!    establishes the provider's identity, and the daemon works with a
//!    provider only when that identity has an enabled record whose
//!    direction is the one the connection was made in.
//! 2. **One connection.** The daemon holds at most one connection per
//!    provider identity at a time.
//! 3. **Changes take effect at once.** [`add`] dials an outgoing
//!    provider; [`edit`] closes the connection when the direction or
//!    the discover flag changes, and the provider is reached again as
//!    the record now stands; [`enable`] dials or admits it again;
//!    [`disable`] stops the dial, refuses a dial in, closes the
//!    connection, and takes down every run on it, each run's log
//!    keeping the reason.
//! 4. **A delete stops nothing.** [`delete`] forgets only a disabled
//!    provider that no agent or tool is pinned to or mounts the volumes
//!    of; its volumes' tags go with it.
//!
//! [`get`] answers one provider by its identity, and [`list`] lists
//! them, each with where the daemon stands with it; [`tag`] and
//! [`untag`] change their tags.
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
//! [`discover`] lists the identities discoverable through its connected
//! providers: what a [`daemons`](crate::daemon::endpoints::daemons)
//! record names a remote by.

mod direction;
mod ip;
mod status;

pub use direction::*;
pub use ip::*;
pub use status::*;

pub mod add;
pub mod delete;
pub mod disable;
pub mod discover;
pub mod edit;
pub mod enable;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
