//! One ask, as it arrived.

use bytes::Bytes;

use super::super::request::{Frame, FrameError};

/// One frame from `/requests`: the channel, and the bytes it came in.
///
/// The bytes are kept rather than the decoded frame, because most of
/// what an ask carries borrows — a vault key, a command's bytes — and
/// an ask has to outlive the message it arrived in to be answered
/// later. [`frame`](Self::frame) decodes on demand, borrowing from
/// here; the stream already decoded once to validate, so it does not
/// fail on an ask the stream handed over.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The channel the container minted for this ask: the answer path
    /// is opened with it.
    pub channel: u32,
    /// The whole frame, as received.
    pub payload: Bytes,
}

impl Ask {
    /// The ask, decoded from its bytes.
    pub fn frame(&self) -> Result<Frame<'_>, FrameError> {
        Frame::decode(&self.payload)
    }
}
