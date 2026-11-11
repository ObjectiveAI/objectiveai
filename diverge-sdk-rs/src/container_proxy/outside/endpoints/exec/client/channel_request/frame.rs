//! What a client's channel request frame carries for an exec: the cancel.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Cancel the command, killing its process group. Nothing in it: any
/// channel request on an exec scope is the cancel, and the scope's finish
/// answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

impl Encode for Frame {
    /// Nothing to write.
    type Error = std::convert::Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Nothing to read.
    type Error = std::convert::Infallible;

    fn decode(_bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Frame)
    }
}
