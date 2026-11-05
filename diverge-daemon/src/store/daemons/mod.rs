//! The remote daemons this daemon reaches: one record per account of
//! ours on one of them.
//!
//! A record is named, and names the remote by the identity it answers a
//! handshake as and our account there by the identity of the host's
//! this daemon presents; it carries a description, tags, when it was
//! added and by whom. Several records may name the same remote. A record
//! a connected tool names is not deleted — see [`in_use`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod daemons;

pub use daemons::*;
