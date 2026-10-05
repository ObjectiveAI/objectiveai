//! What brings a resource into being.

use serde::{Deserialize, Serialize};

/// The actions over resources that make one where there was none, which
/// a grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"upload"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Upload a file or a directory, as
    /// [`resources::upload`](crate::daemon::endpoints::resources::upload)
    /// does.
    Upload,
}
