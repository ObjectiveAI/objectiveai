//! What the six scopes' executors share: the asks a proxy opens on a
//! scope, read as a stream.
//!
//! [`Asks`] reads a scope's inbox — the channel requests the proxy
//! opens on it — and yields each with the proxy's channel number, so
//! whoever answers can quote it back; [`Ask`] is what a begin scope's
//! twelve decode to, owned, so both families' frames become one thing
//! to relay. The rest of what an executor needs — the readers over a
//! channel this end opened, the unary exchange, the error types — is
//! [`containers::client`](crate::provider::endpoints::containers::client)'s,
//! used as the caller's executors use it: the server is the client on
//! this wire.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod ask;
mod asks;

pub use ask::*;
pub use asks::*;
