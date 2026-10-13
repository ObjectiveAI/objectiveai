//! One account of a daemon's, as the document holds it.

use serde::{Deserialize, Serialize};

/// An account the CLI may dial a daemon as: the key the daemon admits
/// it by, and nothing else — the daemon knows which account the key
/// is, and the name here is the document's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// The account's key, as its create gave it.
    pub key: String,
}
