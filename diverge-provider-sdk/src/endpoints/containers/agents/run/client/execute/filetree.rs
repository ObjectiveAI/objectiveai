//! The container's tree, watched.

use bytes::Bytes;

use crate::decode::Decode as _;
use crate::endpoints::containers::agents::run::server::channel_response;
use crate::endpoints::containers::client::{Answered, ChannelStream};
use crate::shared::error::Error;
use crate::shared::filetree;

/// A snapshot, then every change, until the watch ends; an `Error`
/// is the tree that could not be watched, or is no longer.
#[derive(Debug, Clone, Copy)]
pub struct Filetree;

impl Answered for Filetree {
    type Item = filetree::response::Frame;
    type Error = channel_response::filetree::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Self::Item, Error>, Self::Error> {
        Ok(match channel_response::filetree::Frame::decode(&payload)? {
            channel_response::filetree::Frame::Filetree(frame) => Ok(frame),
            channel_response::filetree::Frame::Error(error) => Err(error),
        })
    }
}

/// The changes on the container's tree, for as long as the watch
/// lasts.
pub type FiletreeStream = ChannelStream<Filetree>;
