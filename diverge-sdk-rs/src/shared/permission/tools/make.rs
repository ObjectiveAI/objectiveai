//! What brings a tool into being.

use serde::{Deserialize, Serialize};

/// The actions over tools that make one where there was none, which a
/// grant holds or does not and judges by nothing else. Snake case on
/// the wire: `"create"`, `"register"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Create a tool, as
    /// [`tools::create`](crate::daemon::endpoints::tools::create) does.
    Create,
    /// Hold another daemon's tool under a name, as
    /// [`tools::register`](crate::daemon::endpoints::tools::register)
    /// does.
    Register,
}
