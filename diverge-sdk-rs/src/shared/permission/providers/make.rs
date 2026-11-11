//! What brings a provider's record into being.

use serde::{Deserialize, Serialize};

/// The actions over providers that make one where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Record a provider, as
    /// [`providers::add`](crate::daemon::endpoints::providers::add) does.
    Add,
}
