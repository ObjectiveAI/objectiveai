//! Asking for the agent's schema.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask what the agent value may be.
///
/// Nothing to say: which container is the scope's, and it has one
/// agent, so the payload is empty and the channel opening is the ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request;

/// No bytes at all.
impl Encode for Request {
    /// [`Infallible`](std::convert::Infallible): no bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Request {
    /// [`Infallible`](std::convert::Infallible): there is nothing to
    /// read.
    type Error = std::convert::Infallible;

    /// Whatever bytes arrive are ignored. There are none to send, so a
    /// peer that sent some knows something this version does not, and
    /// leaving room for it is cheaper than refusing it.
    fn decode(_bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Request)
    }
}
