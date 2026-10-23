//! One account of a daemon's, as the document holds it.

use serde::{Deserialize, Serialize};

/// An account the CLI may dial a daemon as: the keyring entry holding
/// the key the daemon admits it by, and nothing else — the daemon
/// knows which account the key is, and the name here is the
/// document's own. The document holds no key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// The name of the entry in the root's
    /// [`keyring`](diverge_sdk::keyring) holding the account's key: the
    /// entry the account's create named, on the daemon's host, or one
    /// `diverge keyring set` kept for a key someone gave you.
    pub key: String,
}
