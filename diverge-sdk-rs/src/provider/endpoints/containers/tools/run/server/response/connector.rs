//! One connector, as the runner is told of it.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A connector attached to the container, or leaving it: the identity
/// the provider authorized its connection under, and the address that
/// connection came from — the same two the runner was asked about,
/// the address attested and the identity the provider's own word.
/// One identity connected twice is two connectors, told of twice each
/// way; nothing here tells the two apart, and nothing needs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connector {
    /// The identity the provider authorized the connector's connection
    /// under.
    pub identity: String,
    /// Where the connector's socket came from, as the provider saw it.
    pub address: IpAddr,
}
