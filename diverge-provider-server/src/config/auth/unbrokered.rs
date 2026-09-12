//! One way of judging an unbrokered credential.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// One way of judging an unbrokered credential: a key the credential
/// must equal, or a hook that judges it.
///
/// Written as an object with either `key` or `authorize_hook`, and
/// nothing else besides what that one allows; an object with both, or
/// with neither, is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Unbrokered {
    /// A credential that must equal `key`, and must come from
    /// `address` when one is written.
    Key {
        /// The string the credential must equal, byte for byte.
        key: String,
        /// The one peer address the key is accepted from. Absent
        /// means any address.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<IpAddr>,
    },
    /// A hook that judges the credential itself.
    Hook {
        /// The hook, by name: the folder `hooks/<name>/` of the
        /// provider's directory, run as [`hook`](crate::hook)
        /// provides, and given the credential and the peer's address.
        authorize_hook: String,
    },
}
