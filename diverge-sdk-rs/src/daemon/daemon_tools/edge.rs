//! One way a transfer may go: from one side to another.

use serde::{Deserialize, Serialize};

use super::{Destination, Source};

/// One edge of the transfer tool's reach: a transfer passes when its
/// source is within `from` and its destination within `to`. A list
/// of these is the whole reach, and a transfer passes when any one
/// of them passes it. Direction is the point: an edge from the
/// container to resources lets it publish, an edge from resources
/// to a tool lets it feed one, and neither lets the other.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// Where a transfer may read: see [`Source`].
    pub from: Source,
    /// Where it may write: see [`Destination`].
    pub to: Destination,
}
