//! Whether the queue held anything.

use std::convert::Infallible;

use bytes::Bytes;

use super::Answered;
use crate::decode::Decode as _;
use crate::shared::containers::dequeue;

/// The outcome of a dequeue, whole — dequeued, empty, or the error.
/// The frame is the answer, `Error` included.
#[derive(Debug, Clone, Copy)]
pub struct Dequeue;

impl Answered for Dequeue {
    type Item = dequeue::response::Frame;
    type Error = dequeue::response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Self::Item, Infallible>, Self::Error> {
        dequeue::response::Frame::decode(&payload).map(Ok)
    }
}
