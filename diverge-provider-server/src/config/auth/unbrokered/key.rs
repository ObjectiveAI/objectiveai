//! A key the credential must equal.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A credential that must equal `key`, and must come from `address`
/// when one is written; a peer that presents it is `identity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    /// The string the credential must equal, byte for byte.
    pub key: String,
    /// Who a peer that presents the key is: the string every handler
    /// and every capability receives as the client's identity. The
    /// key itself never serves as one.
    pub identity: String,
    /// The one peer address the key is accepted from. Absent means
    /// any address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<IpAddr>,
}
