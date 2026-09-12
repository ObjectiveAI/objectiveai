//! The loop's chunks.

use bytes::Bytes;

use super::Answered;
use crate::decode::Decode as _;
use crate::shared::containers::run_loop;
use crate::shared::containers::run_loop::response::AgenticLoopChunk;
use crate::shared::error::Error;

/// One chunk per frame until the loop ends; an `Error` is the loop
/// that never ran, first, or the loop that failed, last.
#[derive(Debug, Clone, Copy)]
pub struct RunLoop;

impl Answered for RunLoop {
    type Item = AgenticLoopChunk;
    type Error = run_loop::response::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<AgenticLoopChunk, Error>, Self::Error> {
        Ok(match run_loop::response::Frame::decode(&payload)? {
            run_loop::response::Frame::Chunk(chunk) => Ok(chunk),
            run_loop::response::Frame::Error(error) => Err(error),
        })
    }
}
