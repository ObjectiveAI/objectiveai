//! What brings a route into being.

use serde::{Deserialize, Serialize};

/// The actions over routes that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"set"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Set a dependency position's route to a tool, as
    /// [`tools::routes::set`](crate::daemon::endpoints::tools::routes::set)
    /// does.
    Set,
}
