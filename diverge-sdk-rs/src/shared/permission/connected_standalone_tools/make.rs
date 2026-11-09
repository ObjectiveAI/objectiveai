//! What brings a connected tool into being.

use serde::{Deserialize, Serialize};

/// The actions over connected tools that make one where there was
/// none, which a grant holds or does not and judges by nothing else.
/// Snake case on the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Hold another daemon's standalone tool under a name, as
    /// [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create)
    /// does, the daemon it is on judged by a `connect` grant over
    /// daemons.
    Create,
}
