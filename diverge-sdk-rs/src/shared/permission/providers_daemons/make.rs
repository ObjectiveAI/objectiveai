//! What brings a daemon into being.

use serde::{Deserialize, Serialize};

/// The actions over daemons that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Add a daemon to connect to, as
    /// [`providers::daemons::add`](crate::daemon::endpoints::providers::daemons::add)
    /// does.
    Add,
}
