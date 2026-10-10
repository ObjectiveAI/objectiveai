//! One thing the run's main stream says after the id.

use crate::provider::endpoints::containers::tools::run::server::response::Connector;

/// A connector came, or went: what the run's main stream carries
/// after the id, one per connect scope the provider admitted and one
/// when that scope ended, however it ended. A caller that counts
/// them knows how many connectors the container has, which is what
/// it holds the container up for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Connection {
    /// A connector is attached to the container.
    Connected(Connector),
    /// A connector's scope on the container has ended.
    Disconnected(Connector),
}
