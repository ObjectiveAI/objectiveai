//! Which of its two forms a judge takes.

use serde::{Deserialize, Serialize};

/// A key judge or a hook judge: the member a
/// [`Judge`](crate::daemon::endpoints::providers::incoming::Judge) is
/// told apart by, as a value of its own to narrow a list by. Snake case
/// on the wire: `"key"`, `"authorize_hook"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A key the credential must equal.
    Key,
    /// A hook that judges the credential.
    AuthorizeHook,
}
