//! Remote daemons: the other daemons this daemon reaches, each record
//! one account of ours on one of them.
//!
//! A record names the remote by the identity it answers as, and the
//! identity of the host's this daemon presents to it, and is named
//! itself: a relay's names, and a connected tool's daemon, are records'
//! names. [`add`] makes one; [`get`] and [`list`] read them, a list kept
//! open; [`edit`] changes one; [`delete`] removes one no connected tool
//! names; [`tag`] and [`untag`] change their tags. The rules:
//!
//! 1. **Several accounts.** Several records may name the same remote:
//!    each is another account of ours there, and the name says which.
//! 2. **Reached through discovery.** The daemon reaches a remote through
//!    a connected provider whose discover listing holds the remote's
//!    identity, one chosen at random where several do, and holds those
//!    listings open while it is reaching remotes, as
//!    [`providers::discover`](crate::daemon::endpoints::providers::discover)
//!    holds them while watched. A remote that is discoverable on no
//!    connected provider is not reached.
//! 3. **One connection per record**, kept while it is used and reused by
//!    every relay and every connected tool naming the record.

mod daemon;

pub use daemon::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
