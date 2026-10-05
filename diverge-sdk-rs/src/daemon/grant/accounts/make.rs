//! What brings an account into being.

use serde::{Deserialize, Serialize};

/// The actions over accounts that make one where there was none, which
/// a grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Create an account, as
    /// [`accounts::create`](crate::daemon::endpoints::accounts::create)
    /// does.
    Create,
}
