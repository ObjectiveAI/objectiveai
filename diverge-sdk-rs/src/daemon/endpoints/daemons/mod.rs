//! Remote daemons: the other daemons this daemon reaches, each record
//! one account of ours on one of them.
//!
//! A record names the remote by the identity it answers as, the
//! identity of the host's this daemon presents to it, and the routes it
//! may be reached by, and is known by the [`DaemonId`] those hash to:
//! a relay's hops, and a connected tool's daemon and route, are records'
//! IDs. There are no names. [`add`] makes one; [`get`] and [`list`] read
//! them, a list kept open; [`edit`] changes one's description;
//! [`delete`] removes one no connected tool names; [`tag`] and [`untag`]
//! change their tags. The rules:
//!
//! 0. **An ID is the target.** A record's ID is the hash of what it
//!    reaches and as whom — see [`DaemonId::of`] — so it is fixed but
//!    for its description and tags: reaching elsewhere, or as another
//!    account, or by other routes, is another record, deleted and added,
//!    with another ID. A hop that resolves an ID resolves exactly the
//!    record the caller meant, or none.
//! 1. **Several accounts.** Several records may name the same remote:
//!    each is another account of ours there, or the same account by
//!    other routes, and its ID says which.
//! 2. **Reached through the listings.** The daemon reaches a remote through
//!    a connected provider whose daemons list holds the remote's
//!    identity — one route, a
//!    [`Via`](crate::daemon::endpoints::providers::daemons::list::server::response::Via)
//!    — and holds those listings open while it is reaching remotes, as
//!    [`providers::daemons::list`](crate::daemon::endpoints::providers::daemons::list)
//!    holds them while watched. A remote that is announced on no
//!    connected provider is not reached.
//! 2a. **Through the routes its record lets through.** Of those routes,
//!    only the ones the record's [`Routes`] let through are candidates,
//!    one chosen at random where several are; none is the remote not
//!    reached, with no fallback to a route the filters refuse.
//! 3. **One connection per record**, kept while it is used and reused by
//!    every relay and every connected tool naming the record.

mod daemon;
mod id;
mod routes;

pub use daemon::*;
pub use id::*;
pub use routes::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
