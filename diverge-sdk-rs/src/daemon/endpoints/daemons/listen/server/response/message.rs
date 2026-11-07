//! One message sent to the daemon, as a listener reads it.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// A message sent to the daemon's identity at one of its providers: the
/// provider it came through, who sent it, and what it says.
///
/// The body is bytes and nothing else: neither the daemon nor the
/// provider reads it, and neither prescribes a format for it. As JSON —
/// for a client that prints what the daemon answered — the body is
/// standard base64.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[derive(schemars::JsonSchema)]
pub struct Message {
    /// The provider the message came through, by its identity.
    pub provider: Identity,
    /// The identity of the sending connection at that provider, as the
    /// provider authorized it.
    pub from: Identity,
    /// The body, exactly as the send carried it.
    #[serde(serialize_with = "crate::shared::containers::exec::response::base64")]
    #[schemars(with = "String")]
    pub body: Bytes,
}

/// The two identities of a [`Message`], as its payload writes them
/// ahead of the body: what the length before them counts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct Head {
    /// The provider it came through.
    pub provider: Identity,
    /// Who sent it.
    pub from: Identity,
}
