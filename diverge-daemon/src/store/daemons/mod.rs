//! The remote daemons this daemon reaches: one record per account of
//! ours on one of them.
//!
//! A record names the remote by the identity it answers a handshake as,
//! our account there by the identity of the host's this daemon presents,
//! and the routes it may be reached by, and is keyed by the
//! [`DaemonId`](diverge_sdk::daemon::endpoints::daemons::DaemonId) those
//! hash to — kept as a `BIGINT`, see [`to_column`]; it carries a
//! description, tags, when it was added and by whom, and only the
//! description and the tags ever change. Several records may name the
//! same remote. A record a connected tool names is not deleted — see
//! [`in_use::daemon`](crate::store::in_use::daemon).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod daemons;

pub use daemons::*;
