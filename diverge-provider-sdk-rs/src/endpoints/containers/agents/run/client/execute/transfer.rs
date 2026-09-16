//! One file, copied into another container: the confirmation.

use bytes::Bytes;

use crate::decode::Decode as _;
use crate::endpoints::containers::agents::run::server::channel_response;
use crate::endpoints::containers::client::Answered;
use crate::shared::error::Error;

/// The file is at the destination in the other container, once; or
/// the `Error` that it is not.
#[derive(Debug, Clone, Copy)]
pub struct Transfer;

impl Answered for Transfer {
    type Item = ();
    type Error = channel_response::transfer::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<(), Error>, Self::Error> {
        Ok(match channel_response::transfer::Frame::decode(&payload)? {
            channel_response::transfer::Frame::Transferred(_) => Ok(()),
            channel_response::transfer::Frame::Error(error) => Err(error),
        })
    }
}
