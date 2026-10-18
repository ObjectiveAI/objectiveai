//! What brings an agent template into being.

use serde::{Deserialize, Serialize};

/// The actions over agent templates that make one where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Make a template, as
    /// [`agents::templates::create`](crate::daemon::endpoints::agents::templates::create)
    /// does.
    Create,
}
