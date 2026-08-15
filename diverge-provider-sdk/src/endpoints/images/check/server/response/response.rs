//! Whether the image can be supplied.

use serde::{Deserialize, Serialize};

use super::{Available, Unavailable};

/// Whether a provider can supply the image that was asked about.
///
/// Untagged, with each variant's payload carrying its own `type`
/// constant — the same discipline the agentic loop chunks use. serde
/// has no tag of its own to read, so the answer goes on the wire as
/// itself rather than as a wrapper around itself.
///
/// Two variants rather than a `bool`, because only one of the two
/// answers has anything more to say. An available image has terms
/// attached to it; an unavailable one is just absent. A boolean would
/// force everything that qualifies availability to sit beside it as an
/// optional field that is meaningless when the answer is no.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// The provider can supply it.
    Available(Available),
    /// It cannot.
    Unavailable(Unavailable),
}
