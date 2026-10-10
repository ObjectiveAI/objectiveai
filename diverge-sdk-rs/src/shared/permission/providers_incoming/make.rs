//! What brings an incoming credential into being.

use serde::{Deserialize, Serialize};

/// The actions over incoming credentials that make one where there was
/// none, which a grant holds or does not and incoming credentials by
/// nothing else. Snake case on the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Add an incoming credential of incoming providers, as
    /// [`providers::incoming::add`](crate::daemon::endpoints::providers::incoming::add)
    /// does.
    Add,
}
