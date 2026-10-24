//! The connections to other daemons, each through a provider both
//! are connected to, with this daemon as the client.
//!
//! A remote daemon is the configuration's: a name, the issuer this
//! daemon presents its identity from, and the links it is reached
//! through. [`connect`] is the one connection to it: the one held,
//! while its router runs; else one opened through the links whose
//! providers are connected now, in random order, by the provider
//! protocol's `daemons::connect` — the remote named by the link's
//! `daemon` identity, the relayed connection opened by the
//! [`handshake`](diverge_sdk::wire::handshake), in which the remote
//! admits this daemon's identity as one of its accounts — the first
//! to answer kept as a [`Peer`] in [`Live`](crate::daemon::Live), by
//! name, until it ends. What a connection is FOR is acting as this
//! daemon on that one as that account: a connected tool, whose
//! `tools::connect` is opened on the peer's handle for the tool's MCP
//! exchanges to travel on for as long as it is held; and a relayed
//! request, put on the peer's handle whole. The other direction — a
//! daemon reaching this one — is
//! [`providers::accept`](crate::providers::accept). [`Fail`] is why no
//! connection could be opened.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connect;
mod fail;
mod peer;

pub use connect::*;
pub use fail::*;
pub use peer::*;
