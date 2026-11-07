//! The connections to other daemons, each through a provider both
//! are connected to, with this daemon as the client.
//!
//! A remote daemon is a [record](crate::store::daemons): its ID, the
//! remote's identity, our account there, and the routes it may be
//! reached by. [`connect`] is the one
//! connection a record names: the one held, while its router runs; else
//! one opened through a connected provider whose discover listing holds
//! the remote, chosen at random among them, by the provider protocol's
//! `daemons::connect` — the relayed connection opened by the
//! [`handshake`](diverge_sdk::wire::handshake), in which the remote
//! admits our account as one of its accounts — kept as a [`Peer`] in
//! [`Live`](crate::daemon::Live), by the record's ID, until it ends. What a connection is FOR is acting as this
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
