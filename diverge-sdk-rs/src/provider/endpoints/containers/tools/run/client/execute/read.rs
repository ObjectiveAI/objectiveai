//! One file, read out of the container.

use bytes::Bytes;

use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::tools::run::server::channel_response;
use crate::provider::endpoints::containers::client::{Answered, ChannelStream};
use crate::shared::error::Error;

/// The file's pieces in order until the finish; an `Error` is the
/// file not read, or not all of it.
#[derive(Debug, Clone, Copy)]
pub struct Read;

impl Answered for Read {
    type Item = Bytes;
    type Error = channel_response::read::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Bytes, Error>, Self::Error> {
        Ok(match channel_response::read::Frame::decode(&payload)? {
            channel_response::read::Frame::Body(body) => Ok(payload.slice_ref(body.0)),
            channel_response::read::Frame::Error(error) => Err(error),
        })
    }
}

/// A file's bytes, in pieces.
pub type ReadStream = ChannelStream<Read>;
