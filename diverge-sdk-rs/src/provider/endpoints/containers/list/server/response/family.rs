//! Which of the two families a listed container belongs to.

use serde::{Deserialize, Serialize};

/// An agent container or a tool container: what the run that made
/// it was. Snake case on the wire: `"agent"`, `"tool"`. A lister that
/// means to [`connect`](crate::provider::endpoints::containers::tools::connect)
/// needs to know, since an agent container takes no connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Made by a [`containers::agents::run`](crate::provider::endpoints::containers::agents::run).
    Agent,
    /// Made by a [`containers::tools::run`](crate::provider::endpoints::containers::tools::run).
    Tool,
}
