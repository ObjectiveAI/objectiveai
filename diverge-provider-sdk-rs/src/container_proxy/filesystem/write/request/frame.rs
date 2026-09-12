//! A chunk of the content.

use std::convert::Infallible;

use crate::encode::{Encode, Writer};

/// One message of content on `/filesystem/write`, every message after the
/// first: bytes, verbatim, in order. An EMPTY one is not content —
/// it is the end of it, after which the container answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the message they arrived in.
    pub &'a [u8],
);

impl Encode for Frame<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one message: the bytes, kept.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Frame(bytes))
    }
}
