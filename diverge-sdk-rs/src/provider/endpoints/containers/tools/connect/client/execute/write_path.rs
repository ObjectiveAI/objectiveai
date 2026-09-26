//! One file, written into the container: the confirmation.

use bytes::Bytes;

use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::tools::connect::server::channel_response;
use crate::provider::endpoints::containers::client::Answered;
use crate::shared::error::Error;

/// The file is at the path, once; or the `Error` that it is not.
#[derive(Debug, Clone, Copy)]
pub struct WritePath;

impl Answered for WritePath {
    type Item = ();
    type Error = channel_response::write_path::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<(), Error>, Self::Error> {
        Ok(match channel_response::write_path::Frame::decode(&payload)? {
            channel_response::write_path::Frame::Written(_) => Ok(()),
            channel_response::write_path::Frame::Error(error) => Err(error),
        })
    }
}
