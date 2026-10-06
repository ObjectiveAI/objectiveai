//! What is done with the database.

use serde::{Deserialize, Serialize};

/// The actions over the database, each the endpoint of the same name
/// under [`postgres`](crate::daemon::endpoints::postgres).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Read which database the daemon serves, a remote URL without its
    /// password.
    Get,
    /// Swap which database the daemon serves, giving a remote URL
    /// whole.
    Set,
    /// List the container connections open through it.
    Connections,
}
