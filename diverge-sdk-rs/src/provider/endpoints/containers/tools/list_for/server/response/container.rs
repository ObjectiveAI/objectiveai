//! One container, as its runner let it be told of.

use serde::{Deserialize, Serialize};

/// One tool container the identity runs, whose runner said the
/// lister may see it: its id.
///
/// The id is the one its run was answered with, the capability a
/// [`Connect`](crate::shared::containers::request::Connect) names.
/// Nothing else of the container is told: not its image, not its
/// mounts, not who is attached. A lister that may join it learns
/// what it needs from inside.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Container {
    /// The container's id, as its run answered it.
    pub id: String,
}
