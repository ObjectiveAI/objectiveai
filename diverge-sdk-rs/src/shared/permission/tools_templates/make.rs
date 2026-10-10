//! What brings a tool template into being.

use serde::{Deserialize, Serialize};

/// The actions over tool templates that make one where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"create"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Make a tool template, as
    /// [`tools::templates::create`](crate::daemon::endpoints::tools::templates::create)
    /// does.
    Create,
}
