//! Providers: the records of the providers the daemon works with.
//!
//! A provider is where containers run and volumes live. Which providers
//! the daemon works with is its records', kept in its database: one per
//! provider [`Identity`](crate::shared::Identity), each with the
//! [`Direction`] its connection is made in — the provider dials the
//! daemon, from the IPs it may, or the daemon dials it — what the
//! daemon announces itself with there, whether it accepts other daemons
//! and hears messages through it, and whether it is enabled. The
//! rules:
//!
//! 1. **By identity.** Whichever end dialled, the mutual handshake
//!    establishes the provider's identity, and the daemon works with a
//!    provider only when that identity has an enabled record whose
//!    direction is the one the connection was made in.
//! 2. **One connection.** The daemon holds at most one connection per
//!    provider identity at a time.
//! 3. **Changes take effect at once.** [`add`] dials an outgoing
//!    provider; [`edit`] changes what the daemon holds open on the
//!    connection as it stands, and closes the connection only when the
//!    record's `ips` no longer admit its source address; [`enable`]
//!    dials or admits it again; [`disable`] stops the dial, refuses a
//!    dial in, closes the connection, and takes down every run on it,
//!    each run's log keeping the reason.
//! 4. **A delete stops nothing.** [`delete`] forgets only a disabled
//!    provider that no agent or tool is pinned to or mounts the volumes
//!    of; its volumes' tags go with it.
//! 5. **Three scopes, per record.** While a provider is connected, the
//!    daemon holds open on it exactly what its record says, each
//!    independent of the other two: the provider's `daemons::announce`,
//!    with the record's `announcement`, when the record has one; its
//!    `daemons::accept` when `accept_daemons` is `true`; and its
//!    `daemons::listen` when `messageable` is `true`. The daemon sends
//!    a message through a provider whatever the three are.
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
//! [`daemons`] holds the three requests over the daemons reached
//! through the providers: [`daemons::list`] lists the identities
//! announced on the connected providers, what a
//! [`daemons`](crate::daemon::endpoints::daemons) record names a remote
//! by; [`daemons::listen`] hears the messages sent to the daemon through
//! the messageable ones; [`daemons::message`] sends one through them.

mod direction;
mod ip;
mod status;

pub use direction::*;
pub use ip::*;
pub use status::*;

pub mod add;
pub mod daemons;
pub mod delete;
pub mod disable;
pub mod edit;
pub mod enable;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
