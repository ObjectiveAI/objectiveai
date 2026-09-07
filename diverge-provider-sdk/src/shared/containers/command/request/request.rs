//! The command, as the container asks it.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A diverge command: bytes in the CLI's own vocabulary, which this
/// layer never reads. The whole payload after whatever tag or kind
/// names the ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a>(
    /// The command, verbatim.
    pub &'a [u8],
);

impl Encode for Request<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// [`Infallible`]: the bytes are the command.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Request(bytes))
    }
}
