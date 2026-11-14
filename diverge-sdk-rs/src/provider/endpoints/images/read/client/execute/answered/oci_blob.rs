//! The blob answer, read.

use std::convert::Infallible;

use bytes::Bytes;

use crate::provider::endpoints::containers::client::Answered;
use crate::shared::containers::oci::blob::response;
use crate::wire::decode::Decode as _;

/// One frame of a blob channel: a piece of the blob, verbatim, a view
/// into the frame it arrived in. Nothing here can fail: a piece is
/// its bytes.
#[derive(Debug, Clone, Copy)]
pub struct OciBlob;

impl Answered for OciBlob {
    type Item = Bytes;
    type Error = Infallible;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Bytes, Infallible>, Infallible> {
        let frame = response::Frame::decode(&payload)?;
        Ok(Ok(payload.slice_ref(frame.body)))
    }
}
