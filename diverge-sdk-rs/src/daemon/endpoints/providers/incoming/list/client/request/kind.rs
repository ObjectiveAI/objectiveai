//! Which of its two kinds a judge is.

use serde::{Deserialize, Serialize};

/// A key judge or a hook judge: the `kind` a
/// [`Judge`](crate::daemon::endpoints::providers::incoming::Judge) is
/// tagged with, as a value of its own to narrow a list by. Snake case
/// on the wire: `"key"`, `"hook"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A key the credential must equal.
    Key,
    /// A hook that judges the credential.
    Hook,
}
