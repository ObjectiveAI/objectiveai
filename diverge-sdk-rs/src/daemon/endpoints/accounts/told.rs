//! A credential without its secret.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A [`Credential`](super::Credential) as a list and a get report it:
/// the same two forms, the key without its key. Untagged JSON, as a
/// credential is: `{"identity":…}` with an `address` if any, or
/// `{"authorize_hook":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Told {
    /// A key credential: the identity it names, and the address it
    /// accepts from, never the key.
    Key {
        /// The identity a client presenting the key has.
        identity: String,
        /// The one peer address the key is accepted from; absent, any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<IpAddr>,
    },
    /// A hook credential: its resource.
    Hook {
        /// The hook's resource, by id.
        authorize_hook: String,
    },
}
