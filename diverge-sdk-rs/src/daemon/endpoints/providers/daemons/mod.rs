//! Daemons: other daemons this daemon reaches, through a provider both
//! are connected to.
//!
//! Which daemons those are is this daemon's configuration's,
//! `daemon.yaml`, read at its start: each a name of the configuration's
//! choosing, the issuer this daemon presents as to it, and its
//! [`Link`]s — one per provider it is reachable through, naming the
//! provider by its identity and the remote daemon by the identity that
//! provider knows it by. No request adds, edits or removes one.
//!
//! Every connection to another daemon goes through a provider — the
//! provider protocol's `daemons::connect`, on a provider a link names,
//! naming the remote by the link's identity — and is a connection like
//! any other: the mutual handshake, this daemon presenting its own
//! identity at the record's issuer, which the remote must hold as one
//! of its accounts. This daemon holds one connection per remote daemon
//! at a time, reused while it lasts, and opens one, when it holds none,
//! through the links whose providers it is connected to, in random
//! order, the first to answer winning.
//!
//! What a daemon record is FOR is acting as this daemon on that one:
//! a connected tool —
//! [`tools::register`](crate::daemon::endpoints::standalone_tools::register)
//! names the record and a tool as that daemon names it, which this
//! daemon reaches through that daemon's
//! [`tools::connect`](crate::daemon::endpoints::standalone_tools::connect)
//! — and a relayed request, any request of this wire carrying a
//! `relay` naming the record, which this daemon puts to that one and
//! carries the answers of. [`get`] answers one by name and [`list`]
//! lists them, with whether a connection to each is held now.

mod link;

pub use link::*;

pub mod get;
pub mod list;
