//! The has answer, read.

use std::convert::Infallible;

use bytes::Bytes;

use crate::provider::endpoints::containers::client::Answered;
use crate::shared::containers::oci::has::response;
use crate::wire::decode::Decode as _;

/// The one frame of a has channel: the repository path the provider
/// holds the image under for the caller, or `None`.
#[derive(Debug, Clone, Copy)]
pub struct OciHas;

impl Answered for OciHas {
    type Item = Option<String>;
    type Error = response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Option<String>, Infallible>, Self::Error> {
        Ok(Ok(response::Frame::decode(&payload)?.name))
    }
}
