//! The manifest answer, read.

use std::convert::Infallible;

use bytes::Bytes;

use crate::provider::client::Manifest;
use crate::provider::endpoints::containers::client::Answered;
use crate::shared::containers::oci::manifest::response;
use crate::wire::decode::Decode as _;

/// The one frame of a manifest channel: the media type and the bytes,
/// owned — the bytes a view into the frame they arrived in.
#[derive(Debug, Clone, Copy)]
pub struct OciManifest;

impl Answered for OciManifest {
    type Item = Manifest;
    type Error = response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Manifest, Infallible>, Self::Error> {
        let frame = response::Frame::decode(&payload)?;
        Ok(Ok(Manifest {
            media_type: frame.media_type.to_string(),
            body: payload.slice_ref(frame.body),
        }))
    }
}
