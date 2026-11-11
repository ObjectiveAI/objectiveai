//! The 32 bytes of the proof.

use std::convert::Infallible;

use super::FrameError;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The proof, verbatim: exactly 32 bytes, no header, no tag —
/// `SHA-256(nonce ‖ D)`, see [`prove`](super::super).
///
/// At most one per channel, then the finish. The empty finish, with
/// no frame before it, is the caller saying it does not hold the
/// image. A frame of any other length than 32 is malformed, and a
/// provider reads it as a failed check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame {
    /// The proof.
    pub proof: [u8; 32],
}

impl Encode for Frame {
    /// Thirty-two bytes copied: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(&self.proof);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// One way to fail: a length other than 32.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        <[u8; 32]>::try_from(bytes)
            .map(|proof| Frame { proof })
            .map_err(|_| FrameError::Length(bytes.len()))
    }
}
