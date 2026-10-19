//! The enum itself.

use serde::{Deserialize, Serialize};

use super::{Hook, Key};

/// One way of judging an unbrokered credential: a key the credential
/// must equal, or a hook that judges it.
///
/// Written as an object with either `key` or `authorize_hook`, and
/// nothing else besides what that one allows; an object with both, or
/// with neither, is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Unbrokered {
    /// A credential that must equal a key.
    Key(Key),
    /// A hook that judges the credential itself.
    Hook(Hook),
}
