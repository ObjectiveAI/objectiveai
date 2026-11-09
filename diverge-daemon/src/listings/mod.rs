//! Listings: the identities listable through the daemon's
//! connected providers, merged.
//!
//! Each provider answers its own `daemons::list`: the identities
//! connected to it whose authorization frame carried the listable flag
//! `1`. The daemon holds exactly one such listing per connected
//! provider while anything watches — opened at the first watcher,
//! stopped after the last — shared by every watcher, and merges them
//! into one item per identity, the providers it was seen on listed
//! with it: [`Listings`], read by
//! [`providers::daemons::list`](crate::serve::providers::daemons::list).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod listings;

pub use listings::*;
