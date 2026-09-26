//! Asking to hear what a server says.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Open the notification stream.
///
/// It carries nothing, and MCP is the reason rather than an economy.
/// Every other exchange here is a JSON-RPC method, and a request naming
/// one carries its params. This is not a method: in Streamable HTTP a
/// client opens the notification stream with a bare `GET` on the same
/// url it POSTs everything else to — no method name, no body, nothing
/// to say. There is no `notifications/subscribe` to mirror.
///
/// So the empty payload is the request, whole.
///
/// # It exists anyway
///
/// Rather than the channel request simply carrying no payload for this
/// one case. A frame that names five things should name them the same
/// way, and a variant with nothing in it is a variant a reader has to
/// check twice — once for what it means, once for why it is shaped
/// differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request;

/// Nothing at all. Which channel it arrives on says what it is, and
/// there is nothing else to say.
impl Encode for Request {
    /// [`Infallible`]: no bytes.
    type Error = Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Infallible> {
        Ok(())
    }
}

impl Decode<'_> for Request {
    /// [`Infallible`]: nothing is read, so nothing can be wrong.
    type Error = Infallible;

    /// Whatever bytes are there are ignored rather than rejected. There
    /// is nothing this could carry, so a reader that found something
    /// has met a writer from a version that gave it one — and the
    /// channel already said what was meant.
    fn decode(_bytes: &[u8]) -> Result<Self, Infallible> {
        Ok(Request)
    }
}
