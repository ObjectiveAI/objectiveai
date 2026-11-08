//! The prove answer, read.

use std::convert::Infallible;

use bytes::Bytes;

use crate::provider::endpoints::containers::client::Answered;
use crate::shared::containers::oci::prove::response;
use crate::wire::decode::Decode as _;

/// The one frame of a prove channel: the 32 bytes of the proof. A
/// frame of any other length does not parse.
#[derive(Debug, Clone, Copy)]
pub struct OciProve;

impl Answered for OciProve {
    type Item = [u8; 32];
    type Error = response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<[u8; 32], Infallible>, Self::Error> {
        let frame = response::Frame::decode(&payload)?;
        Ok(Ok(frame.proof))
    }
}
