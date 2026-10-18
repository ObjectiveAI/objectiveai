//! Whether the container holds the secrets' values it was fed.

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::secrets::feed;
use crate::shared::error::Error;

/// The feed taken, once; or the far end's refusal, which is the
/// activation's error.
#[derive(Debug, Clone, Copy)]
pub struct Secrets;

impl Answered for Secrets {
    type Item = ();
    type Error = feed::response::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<(), Error>, Self::Error> {
        Ok(match feed::response::Frame::decode(&payload)? {
            feed::response::Frame::Fed => Ok(()),
            feed::response::Frame::Error(error) => Err(error),
        })
    }
}
