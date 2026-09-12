//! A chunk of pgwire from the driver, toward the database.

use std::convert::Infallible;

use crate::encode::{Encode, Writer};

/// One message on `/postgres/{channel}` from the driver, toward the database: pgwire bytes,
/// verbatim, never parsed. A pgwire message larger than one of these
/// spans several, and the far end reassembles as from a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
