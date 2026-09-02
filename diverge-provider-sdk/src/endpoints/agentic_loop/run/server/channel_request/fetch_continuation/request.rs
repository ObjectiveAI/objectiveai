//! Asking for the continuation, which needs no naming.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Open the continuation fetch.
///
/// It carries nothing, because there is nothing to choose: a run
/// resumes from the one continuation its caller holds for it, and
/// the channel opening is the whole of the ask. The client answers
/// with the bytes —
/// [`fetch_continuation::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_continuation::Frame)s
/// appending — or with the empty finish, which is a fresh start.
///
/// # It exists anyway
///
/// Rather than the channel request simply carrying no payload for
/// this one case. A frame that names its variants should name them
/// the same way, and a variant with nothing in it is a variant a
/// reader has to check twice — once for what it means, once for why
/// it is shaped differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request;

/// Nothing at all. The tag that says which request this is belongs
/// to whichever frame carries it, and there is nothing else to say.
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

    /// Whatever bytes are there are ignored rather than rejected.
    /// There is nothing this could carry, so a reader that found
    /// something has met a writer from a version that gave it one —
    /// and the channel already said what was meant.
    fn decode(_bytes: &[u8]) -> Result<Self, Infallible> {
        Ok(Request)
    }
}
