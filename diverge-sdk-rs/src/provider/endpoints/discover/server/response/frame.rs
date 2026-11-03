//! What a server's response frame carries for a discover request.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Every identity connected to the provider whose discover flag was
/// `1`.
///
/// One of these on channel `0`, then the scope finishes. The payload
/// is a JSON array of identities, and nothing else — no tag, because
/// there is nothing to discriminate, and no object around it, because
/// there is nothing to say beside the list.
///
/// ```json
/// [{"issuer":"https://issuer.example","subject":"daemon"}]
/// ```
///
/// The rules:
///
/// 1. **Connected, and discoverable.** An identity is listed exactly
///    when a connection under it is open to the provider and that
///    connection's authorization frame carried the flag `1` —
///    whichever end dialled. The asking identity is listed when it is
///    itself discoverable.
/// 2. **Once each.** An identity holds at most one connection to a
///    provider, so it is listed at most once.
/// 3. **In order.** Sorted by issuer, then by subject, byte-wise.
/// 4. **A moment's answer.** The listing is what was connected when the
///    provider answered; nothing follows it when that changes.
/// 5. **No failure.** A provider always knows what is connected to it.
///    None connected and discoverable is the empty list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Frame(pub Vec<Identity>);

/// The array as JSON, and nothing in front of it.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Bytes that are not this array's JSON.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
