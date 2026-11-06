//! Asking for a session to be closed.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Close the session.
///
/// It carries nothing: the session it closes is the one its
/// [`Sessioned`](crate::shared::mcp::session::Sessioned) envelope
/// names, and there is nothing else to say about closing it.
///
/// # It exists anyway
///
/// For the reason the notifications request gives: a frame that names
/// its exchanges should name them all the same way, so the channel
/// request carries a request here as it does everywhere else.
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

    /// Whatever bytes are there are ignored rather than rejected, as
    /// the notifications request ignores them.
    fn decode(_bytes: &[u8]) -> Result<Self, Infallible> {
        Ok(Request)
    }
}
