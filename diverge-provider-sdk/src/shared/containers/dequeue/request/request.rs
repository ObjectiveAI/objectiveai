//! Clearing the queue.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Withdraw every message still waiting in the queue.
///
/// The whole queue, deliberately: enqueued messages carry no
/// identifiers, so there is no vocabulary for withdrawing one — and
/// the caller who wants some messages kept and others gone can clear
/// everything and enqueue again what it still means.
///
/// Each message withdrawn is also answered on ITS channel — the
/// pending enqueues finish with
/// [`Dequeued`](crate::shared::containers::enqueue::response::Frame::Dequeued)
/// — and this channel's own answer says only whether there was
/// anything to withdraw. A message the agent already took stays
/// taken: dequeuing is not un-delivery.
///
/// # It carries nothing
///
/// The empty payload is the request, whole. It exists as a type
/// anyway rather than the frame carrying no payload for this one
/// case: a frame that names its variants should name them the same
/// way, and a variant with nothing in it is a variant a reader has
/// to check twice.
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
    /// [`Infallible`]: whatever follows the tag, this is what it
    /// means.
    type Error = Infallible;

    fn decode(_bytes: &[u8]) -> Result<Self, Infallible> {
        Ok(Request)
    }
}
