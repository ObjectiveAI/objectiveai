//! The command, as the container asks it.

use std::convert::Infallible;

use crate::encode::{Encode, Writer};

/// A diverge command: bytes in the CLI's own vocabulary, which this
/// layer never reads. Kind `10` on `/requests`, the whole payload
/// after the kind; answered on `/command/{channel}`.
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
