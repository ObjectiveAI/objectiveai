//! What is done to identities on the host.

use serde::{Deserialize, Serialize};

/// The actions over identities on the host, which a grant reaches as
/// far as its `within` says. Snake case on the wire: `"list"`,
/// `"login"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// List them, as
    /// [`identities::list`](crate::daemon::endpoints::identities::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Sign one in again, replacing it, as
    /// [`identities::login`](crate::daemon::endpoints::identities::login)
    /// does for an identity already on the host.
    Login,
}
