//! Accepting connections from other daemons, through the provider.
//!
//! A daemon opens the scope with the tag and nothing else. The
//! provider answers exactly one response: the identity the daemon's
//! connection is authorized under — what connectors name it by, and
//! what it names this provider by to them — or an error, when it
//! accepts here already. Then the scope stays open, and every
//! `daemons::connect` naming the identity arrives on it as one
//! channel the provider opens, the provider's half of that
//! connection, carrying who the connector is; the daemon declines by
//! finishing it with nothing, or takes it by opening its own half, a
//! channel of its own quoting the connection's id. The daemon's half
//! carries the connector's client frames, the provider's half the
//! daemon's server frames — a daemon connection as a pair of channels,
//! as a container program's is. The scope ends by the daemon's stop or
//! its connection ending, and every connection relayed on it ends
//! with it.
//!
//! Split by who SENDS: [`client`] is the daemon's traffic, [`server`]
//! the provider's.

pub mod client;
pub mod server;
