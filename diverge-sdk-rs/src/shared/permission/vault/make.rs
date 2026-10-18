//! What brings a vault key into being.

use serde::{Deserialize, Serialize};

/// The actions over the vault that make a key where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"insert"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Insert into a key the vault does not hold, as
    /// [`vault::insert`](crate::daemon::endpoints::vault::insert) does
    /// where the key is new; over a key it holds, the same request is
    /// [`Over::Insert`](super::Over::Insert).
    Insert,
}
