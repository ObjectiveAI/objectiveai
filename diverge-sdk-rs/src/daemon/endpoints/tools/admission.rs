//! One admission on a tool: who may see it or join it from outside.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::Admits;

/// An admission: an identity on the tool's provider, the one address it
/// may come from if one, and what it admits. The KEY is not here, on
/// any request or in any report: for an admission that admits a
/// connect, the daemon mints it when the admission is made, answers it
/// in that response, once, and never reports it again; a connector
/// presents it byte for byte as its `authorization`, and the daemon
/// knows it by the key alone. One admission per identity per tool.
///
/// # Two questions, two claims
///
/// A provider asks the daemon two things about a tool the daemon runs,
/// and the admission answers each by a different claim. For a LIST the
/// provider says who asks — the identity the lister's connection to the
/// provider was authorized under, the provider's own word — and the
/// admission's `identity` is what that is compared with. For a CONNECT
/// the provider relays what the connector wrote as its `authorization`,
/// unread, and the admission's key is what that is compared with: the
/// identity names the admission and is not what admits the connector.
/// An address, when given, bounds both: it is where the provider saw
/// the lister's or the connector's socket come from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Admission {
    /// The identity on the tool's provider: what a lister is judged by,
    /// and what names the admission on a tool. Compared and not read.
    pub identity: String,
    /// The one peer address the lister or the connector is accepted
    /// from, as the provider reports it; absent, any address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<IpAddr>,
    /// What the admission admits: see [`Admits`].
    pub admits: Admits,
}
