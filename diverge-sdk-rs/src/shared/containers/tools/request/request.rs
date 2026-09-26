//! The tools a container declared, as the provider relays them.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Error;

use super::super::Tool;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The list, as the container's proxy answered `Begun` with it.
///
/// Borrowed where the provider encodes it — the list is the
/// [`Begun`](crate::container_proxy::outside::endpoints::tools::begin::server::response::Frame::Begun)'s
/// for the run's life — and owned where the caller decodes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request<'a> {
    /// The tools, in the order the program declared them.
    pub tools: Cow<'a, [Tool]>,
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request<'_> {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request<'_> {
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
