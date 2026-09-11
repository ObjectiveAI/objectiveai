//! The announcement.

use std::convert::Infallible;

use crate::encode::{Encode, Writer};

/// A database connection the driver just opened. Kind `11` on
/// `/requests`, carrying nothing: the channel is the whole ask, and
/// the server opens `/postgres/{channel}` for it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Request;

impl Encode for Request {
    /// [`Infallible`]: nothing is written.
    type Error = Infallible;

    fn encode(&self, _: &mut Writer<'_>) -> Result<(), Infallible> {
        Ok(())
    }
}

impl Request {
    /// Decode from the bytes after the ask's kind: there are none to
    /// read, and any that are there are ignored.
    pub fn decode(_: &[u8]) -> Result<Self, Infallible> {
        Ok(Request)
    }
}
