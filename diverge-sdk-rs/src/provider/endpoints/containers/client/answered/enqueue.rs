//! A message's fate.

use std::convert::Infallible;

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::enqueue;

/// The fate of an enqueued message, whole — delivered, dequeued, or
/// the error — whenever it is known. The frame is the
/// answer, `Error` included: nothing refuses an enqueue, it is only
/// fated.
#[derive(Debug, Clone, Copy)]
pub struct Enqueue;

impl Answered for Enqueue {
    type Item = enqueue::response::Frame;
    type Error = enqueue::response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Self::Item, Infallible>, Self::Error> {
        enqueue::response::Frame::decode(&payload).map(Ok)
    }
}
