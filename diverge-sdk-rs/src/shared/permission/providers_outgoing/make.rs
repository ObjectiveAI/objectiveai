//! What brings an outgoing provider into being.

use serde::{Deserialize, Serialize};

/// The actions over outgoing providers that make one where there was
/// none, which a grant holds or does not and judges by nothing else.
/// Snake case on the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Add a provider to dial, as
    /// [`providers::outgoing::add`](crate::daemon::endpoints::providers::outgoing::add)
    /// does.
    Add,
}
