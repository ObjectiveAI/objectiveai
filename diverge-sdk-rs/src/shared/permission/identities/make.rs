//! What brings an identity onto the host.

use serde::{Deserialize, Serialize};

/// The actions over identities that make one where there was none,
/// which a grant holds or does not and judges by nothing else. Snake
/// case on the wire: `"login"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// Sign in an identity not on the host yet, as
    /// [`identities::login`](crate::daemon::endpoints::identities::login)
    /// does.
    Login,
}
