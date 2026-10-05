//! What brings an agent into being.

use serde::{Deserialize, Serialize};

/// The actions over agents that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Create an agent, as
    /// [`agents::create`](crate::daemon::endpoints::agents::create)
    /// does.
    Create,
}
