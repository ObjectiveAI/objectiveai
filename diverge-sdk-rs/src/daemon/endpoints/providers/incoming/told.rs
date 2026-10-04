//! A judge without its secret.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A [`Judge`](super::Judge) as a list and a get report it: the same
/// two kinds, the key judge without its key. JSON-tagged by `kind`,
/// `key` or `hook`, as a judge is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Told {
    /// A key judge: the identity it names, and the address it accepts
    /// from, never the key.
    Key {
        /// The identity a peer presenting the key has.
        identity: String,
        /// The one peer address the key is accepted from; absent, any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<IpAddr>,
    },
    /// A hook judge: its resource.
    Hook {
        /// The hook's resource, by id.
        resource: String,
    },
}
