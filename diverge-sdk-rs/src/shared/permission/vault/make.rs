//! What brings a vault key into being.

use serde::{Deserialize, Serialize};

/// The actions over the vault that make a key where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"set"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Set a key the vault does not hold, as
    /// [`vault::set`](crate::daemon::endpoints::vault::set) does
    /// where the key is new; over a key it holds, the same request is
    /// [`Over::Set`](super::Over::Set).
    Set,
}
