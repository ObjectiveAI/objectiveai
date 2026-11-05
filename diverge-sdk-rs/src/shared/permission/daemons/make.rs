//! What brings a remote daemon's record into being.

use serde::{Deserialize, Serialize};

/// The actions over remote daemons that make one where there was none,
/// which a grant holds or does not and judges by nothing else. Snake case
/// on the wire: `"add"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Record a remote daemon, as
    /// [`daemons::add`](crate::daemon::endpoints::daemons::add) does.
    Add,
}
