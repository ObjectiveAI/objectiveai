//! The key whose messages are withdrawn.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Withdraw every message enqueued under a key and not yet taken.
///
/// The key is the one an
/// [`enqueue`](crate::shared::containers::enqueue::request::Request::key)
/// carried. Every waiting message under it is withdrawn — there may
/// be several, or none — and the answer says which. The provider and
/// the proxy compare the key and do not read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The key, as the enqueues gave it.
    pub key: String,
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
