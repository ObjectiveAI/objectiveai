//! What brings a judge into being.

use serde::{Deserialize, Serialize};

/// The actions over judges that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Add a judge of incoming providers, as
    /// [`providers::incoming::add`](crate::daemon::endpoints::providers::incoming::add)
    /// does.
    Add,
}
