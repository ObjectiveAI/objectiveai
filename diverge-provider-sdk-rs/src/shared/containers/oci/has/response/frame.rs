//! One byte: held, or not.

use std::convert::Infallible;

use super::FrameError;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Whether the caller holds the image: the byte `1` when it does and
/// the byte `0` when it does not, and nothing after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame {
    /// Held.
    pub held: bool,
}

impl Encode for Frame {
    /// One byte: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(&[u8::from(self.held)]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// No byte, a byte that is neither, or more than one.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        match bytes {
            [] => Err(FrameError::Empty),
            [0] => Ok(Frame { held: false }),
            [1] => Ok(Frame { held: true }),
            [byte] => Err(FrameError::Byte(*byte)),
            _ => Err(FrameError::Trailing),
        }
    }
}
