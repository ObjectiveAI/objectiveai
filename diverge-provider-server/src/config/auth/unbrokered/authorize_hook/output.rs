//! The document the hook writes.

use serde::{Deserialize, Serialize};

/// What the hook writes to stdout, as one JSON document, on exit `0`.
///
/// Exactly one of the two keys is present. `identity` accepts the
/// credential and names the peer: the string every handler and every
/// capability receives as the client's identity. `refused` refuses
/// it, and carries the reason for the provider's log; nothing of it
/// reaches the peer.
///
/// ```json
/// {"identity": "acme"}
/// ```
///
/// ```json
/// {"refused": "no such key"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    /// The credential is accepted, and this is who presented it.
    Accepted(Accepted),
    /// The credential is refused, and this is why.
    Refused(Refused),
}

/// `{"identity": …}`: accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accepted {
    /// The peer's identity.
    pub identity: String,
}

/// `{"refused": …}`: refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refused {
    /// Why, for the provider's log.
    pub refused: String,
}
