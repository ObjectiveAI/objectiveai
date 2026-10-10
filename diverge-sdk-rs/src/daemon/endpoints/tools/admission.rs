//! One admission on a tool: who may join it from outside.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// An admission: an identity on the tool's provider, and the one
/// address it may come from if one. The KEY is not here, on any
/// request or in any report: the daemon mints it when the admission
/// is made, answers it in that response, once, and never reports it
/// again; a connector presents it byte for byte as its
/// `authorization`, and the daemon knows it by the key alone. One
/// admission per identity per tool.
///
/// # What the identity is for
///
/// A provider asks the daemon one thing about a tool the daemon runs:
/// whether a connector may attach, relaying what the connector wrote
/// as its `authorization`, unread. The admission's key is what that
/// is compared with; the identity names the admission on the tool and
/// is not what admits the connector. An address, when given, bounds
/// the admission: it is where the provider saw the connector's socket
/// come from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Admission {
    /// The identity on the tool's provider: what names the admission
    /// on a tool. Compared and not read.
    pub identity: String,
    /// The one peer address the connector is accepted from, as the
    /// provider reports it; absent, any address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<IpAddr>,
}
