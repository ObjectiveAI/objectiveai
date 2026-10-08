//! The agent's conversation, off the run's main stream.

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, stream};

use super::Event;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::agents::run::server::response;
use crate::provider::endpoints::containers::client::{Decoded, Scoped, WaitError};

/// Everything the run's main stream says after the id, for as long as
/// the run lives: every chunk the agent produces, and the proxy's word
/// before a loop's first chunk and after its last — each one
/// [`Event`], as [`response::Frame`] carries them.
///
/// Zero or more [`Ok`], in the order the provider relayed them; then
/// either the end — the finish, the run over as it should — or exactly
/// one [`Err`] and the end. Every error is terminal, and it is the
/// same ending [`ExecuteHandle::wait`](super::ExecuteHandle::wait)
/// reports. There is no timeout: quiet between an
/// [`Active`](Event::Active) and its [`Inactive`](Event::Inactive) is
/// an agent working, and quiet after an `Inactive` is an agent with
/// nothing to say until the next message.
///
/// # This is the main stream's reader
///
/// Nothing else reads the run's main stream. A run whose conversation
/// nobody polls grows a queue nobody reads, and its end is never
/// heard — `wait` waits for this. A caller that wants the end and not
/// the conversation drains this to the end.
#[must_use = "a conversation that is not polled grows a queue nobody reads, and the run's end is never heard"]
pub struct ExecuteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, WaitError<response::FrameError>>> + Send>>,
}

impl ExecuteStream {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        let inner = stream::unfold((scoped, false), |(scoped, done)| async move {
            if done {
                return None;
            }
            match scoped.read(decode).await {
                Ok(Some(event)) => Some((Ok(event), (scoped, false))),
                Ok(None) => None,
                Err(error) => Some((Err(error), (scoped, true))),
            }
        });
        ExecuteStream {
            inner: Box::pin(inner),
        }
    }
}

/// One main-stream frame as the run's response frame: a chunk and the
/// two words are items, the provider's error is the end, and the id
/// or the refusal — forbidden after the first response — is read
/// past.
fn decode(payload: &[u8]) -> Result<Decoded<Event>, response::FrameError> {
    Ok(match response::Frame::decode(payload)? {
        response::Frame::Chunk(chunk) => Decoded::Item(Event::Chunk(chunk)),
        response::Frame::Active => Decoded::Item(Event::Active),
        response::Frame::Inactive => Decoded::Item(Event::Inactive),
        response::Frame::Error(error) => Decoded::Error(error),
        response::Frame::Id(_) | response::Frame::VolumeHeld(_) | response::Frame::VolumeMode(_) => Decoded::Skip,
    })
}

impl Stream for ExecuteStream {
    type Item = Result<Event, WaitError<response::FrameError>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl fmt::Debug for ExecuteStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ExecuteStream")
    }
}
