//! What brings a tool into being.

use serde::{Deserialize, Serialize};

/// The actions over tools that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"create"`. Holding another daemon's tool is the
/// [`connected_standalone_tools`](crate::shared::permission::connected_standalone_tools)
/// kind's `create`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Create a tool, as
    /// [`tools::create`](crate::daemon::endpoints::standalone_tools::create) does.
    Create,
}
