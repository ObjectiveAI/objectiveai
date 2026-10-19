//! What brings a volume into being.

use serde::{Deserialize, Serialize};

/// The actions over volumes that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Create a volume on a provider, as
    /// [`volumes::create`](crate::daemon::endpoints::volumes::create)
    /// does.
    Create,
}
