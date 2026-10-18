//! Connecting to another daemon, through the provider.
//!
//! A daemon opens the scope naming the daemon it wants by the identity
//! the provider knows it by, and the mode it authenticates to it in.
//! The provider finds the acceptor's accept scope, opens its half of
//! the connection on it, and waits for the acceptor's own: a daemon
//! not accepting here, or one that declines, is the scope's one
//! error, then the finish. Else exactly one response says the
//! connection is open, and from then on the scope's responses are
//! the acceptor's server frames, one each, until the acceptor hangs
//! up, which is the finish. The connector's own frames go the other
//! way on the one channel the provider opens on the scope: the
//! connector answers it with its client frames, one each, and
//! finishes it to hang up.
//!
//! Split by who SENDS: [`client`] is the connector's traffic,
//! [`server`] the provider's.

pub mod client;
pub mod server;
