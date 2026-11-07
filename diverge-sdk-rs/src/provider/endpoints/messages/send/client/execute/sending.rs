//! A send opened and not yet answered.

use std::fmt;

use super::super::super::server::response;
use super::Cancel;
use crate::wire::decode::Decode as _;
use crate::wire::frame;

/// A message sent and not yet answered: [`wait`](Self::wait) reads the
/// one answer, and [`cancel`](Self::cancel) takes the message back
/// while it waits — after which the answer, or the finish with nothing
/// before it, says which came first.
#[must_use = "a scope that is not waited on grows a queue nobody reads"]
#[derive(Debug)]
pub struct Sending {
    /// The cancel, to hold apart from the wait.
    cancel: Cancel,
    /// The scope's responses.
    receiver: crate::wire::credit::Receiver,
}

impl Sending {
    /// Over the scope's responses.
    pub(super) fn new(cancel: Cancel, receiver: crate::wire::credit::Receiver) -> Self {
        Sending { cancel, receiver }
    }

    /// The cancel, to hold apart from the wait: a caller that waits on
    /// one task and cancels from another clones this.
    pub fn cancel(&self) -> &Cancel {
        &self.cancel
    }

    /// Wait for the one answer. `None` is the finish with no response
    /// before it: the cancel in time — the receiver never saw the
    /// message — or a request the provider did not serve.
    pub async fn wait(mut self) -> Result<Option<response::Frame>, WaitError> {
        let bytes = self.receiver.recv().await.ok_or(WaitError::Closed)?;
        match frame::server::ServerFrame::decode(&bytes).map_err(WaitError::Frame)? {
            frame::server::ServerFrame::Response { payload, .. } => response::Frame::decode(payload).map(Some).map_err(WaitError::Response),
            frame::server::ServerFrame::ResponseFinish { .. } => Ok(None),
            _ => Err(WaitError::Misrouted),
        }
    }
}

/// A send whose answer could not be read.
#[derive(Debug)]
pub enum WaitError {
    /// The connection ended before the answer.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// A frame that cannot be on a main stream arrived.
    Misrouted,
    /// The answer did not decode as a send response.
    Response(response::FrameError),
}

impl fmt::Display for WaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaitError::Closed => f.write_str("the connection ended before the send was answered"),
            WaitError::Frame(error) => write!(f, "frame did not decode: {error}"),
            WaitError::Misrouted => f.write_str("a frame that cannot be on a main stream arrived"),
            WaitError::Response(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for WaitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WaitError::Frame(error) => Some(error),
            WaitError::Response(error) => Some(error),
            WaitError::Closed | WaitError::Misrouted => None,
        }
    }
}
