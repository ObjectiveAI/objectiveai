//! Discovery: the identities discoverable through the daemon's
//! connected providers, merged.
//!
//! Each provider answers its own `discover` listing: the identities
//! connected to it whose authorization frame carried the discover flag
//! `1`. The daemon holds exactly one such listing per connected
//! provider while anything watches — opened at the first watcher,
//! stopped after the last — shared by every watcher, and merges them
//! into one item per identity, the providers it was seen on listed
//! with it: [`Discovery`], read by
//! [`providers::daemons::discover`](crate::serve::providers::daemons::discover).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod discovery;

pub use discovery::*;
