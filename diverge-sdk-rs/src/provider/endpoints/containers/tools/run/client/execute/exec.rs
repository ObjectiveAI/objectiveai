//! One command, run in the container.

use bytes::Bytes;

use crate::provider::endpoints::containers::client::{Answered, ChannelStream};
use crate::provider::endpoints::containers::tools::run::server::channel_response;
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;

/// How an exec channel's responses are read: every frame of the
/// command's stream an item, its `Error` among them, and the finish the
/// end.
#[derive(Debug, Clone, Copy)]
pub struct Exec;

impl Answered for Exec {
    type Item = channel_response::exec::Owned;
    type Error = channel_response::exec::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Self::Item, Error>, Self::Error> {
        Ok(Ok(channel_response::exec::Frame::decode(&payload)?.owned(&payload)))
    }
}

/// A command's stream, as the provider relays it.
pub type ExecStream = ChannelStream<Exec>;
