//! The agent's conversation, off the run's main stream.

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, stream};

use crate::decode::Decode as _;
use crate::endpoints::containers::agents::run::server::response::{self, AgenticLoopChunk};
use crate::endpoints::containers::client::{Decoded, Scoped, WaitError};

/// Every chunk the agent produces, for as long as the run lives: the
/// scope's main stream after the id, as
/// [`response::Frame::Chunk`] carries it.
///
/// Zero or more [`Ok`], each the [`AgenticLoopChunk`] the provider
/// relayed, in the order the agent produced them; then either the end
/// — the finish, the run over as it should — or exactly one [`Err`]
/// and the end. Every error is terminal, and it is the same ending
/// [`ExecuteHandle::wait`](super::ExecuteHandle::wait) reports. There
/// is no marker between turns and no timeout: quiet is an agent with
/// nothing to say.
///
/// # This is the main stream's reader
///
/// Nothing else reads the run's main stream. A run whose conversation
/// nobody polls grows a queue nobody reads, and its end is never
/// heard — `wait` waits for this. A caller that wants the end and not
/// the conversation drains this to the end.
#[must_use = "a conversation that is not polled grows a queue nobody reads, and the run's end is never heard"]
pub struct ExecuteStream {
    inner: Pin<Box<dyn Stream<Item = Result<AgenticLoopChunk, WaitError<response::FrameError>>> + Send>>,
}

impl ExecuteStream {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        let inner = stream::unfold((scoped, false), |(scoped, done)| async move {
            if done {
                return None;
            }
            match scoped.read(decode).await {
                Ok(Some(chunk)) => Some((Ok(chunk), (scoped, false))),
                Ok(None) => None,
                Err(error) => Some((Err(error), (scoped, true))),
            }
        });
        ExecuteStream {
            inner: Box::pin(inner),
        }
    }
}

/// One main-stream frame as the run's response frame: a chunk is the
/// item, the provider's error is the end, and the id or the refusal —
/// forbidden after the first response — is read past.
fn decode(payload: &[u8]) -> Result<Decoded<AgenticLoopChunk>, response::FrameError> {
    Ok(match response::Frame::decode(payload)? {
        response::Frame::Chunk(chunk) => Decoded::Item(chunk),
        response::Frame::Error(error) => Decoded::Error(error),
        response::Frame::Id(_) | response::Frame::VolumeHeld(_) => Decoded::Skip,
    })
}

impl Stream for ExecuteStream {
    type Item = Result<AgenticLoopChunk, WaitError<response::FrameError>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl fmt::Debug for ExecuteStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ExecuteStream")
    }
}
