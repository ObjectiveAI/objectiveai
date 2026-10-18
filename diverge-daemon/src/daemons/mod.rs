//! The connections to other daemons, each through a provider both
//! are connected to, with this daemon as the client.
//!
//! A daemon record names another daemon the caller holds accounts
//! on: the links it is reached through, and its accounts, each the
//! mode this daemon authenticates to it in as that account.
//! [`connect`] is the one connection to it as one account: the one
//! held, while its router runs; else one opened through the links
//! whose providers are connected now, in random order, by the
//! provider protocol's `daemons::connect` — the remote named by the
//! link's identity, the account's mode presented as the connection's
//! credential, which the remote judges as it judges any client's —
//! the first to answer kept as a [`Peer`] in
//! [`Live`](crate::daemon::Live), by daemon and account, until it
//! ends. What a connection is FOR is acting as this daemon on that
//! one as that account: a connected tool, whose `tools::connect` is
//! opened on the peer's handle for the tool's MCP exchanges to travel
//! on for as long as it is held; and a relayed request, put on the
//! peer's handle whole. The other direction — a daemon reaching this
//! one — is
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
