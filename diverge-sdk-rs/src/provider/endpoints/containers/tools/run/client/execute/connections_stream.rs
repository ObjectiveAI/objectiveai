//! The connectors coming and going, off the run's main stream.

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, stream};

use super::Connection;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::tools::run::server::response;
use crate::provider::endpoints::containers::client::{Decoded, Scoped, WaitError};

/// Everything the run's main stream says after the id, for as long as
/// the run lives: a connector attached, a connector gone — each one
/// [`Connection`], as [`response::Frame`] carries them.
///
/// Zero or more [`Ok`], in the order the provider sent them; then
/// either the end — the finish, the run over as it should — or exactly
/// one [`Err`] and the end. Every error is terminal, and it is the
/// same ending [`ExecuteHandle::wait`](super::ExecuteHandle::wait)
/// reports.
///
/// # This is the main stream's reader
///
/// Nothing else reads the run's main stream. A run whose connections
/// nobody polls grows a queue nobody reads, and its end is never
/// heard — `wait` waits for this. A caller that wants the end and not
/// the connections drains this to the end.
#[must_use = "connections that are not polled grow a queue nobody reads, and the run's end is never heard"]
pub struct ConnectionsStream {
    inner: Pin<Box<dyn Stream<Item = Result<Connection, WaitError<response::FrameError>>> + Send>>,
}

impl ConnectionsStream {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        let inner = stream::unfold((scoped, false), |(scoped, done)| async move {
            if done {
                return None;
            }
            match scoped.read(decode).await {
                Ok(Some(connection)) => Some((Ok(connection), (scoped, false))),
                Ok(None) => None,
                Err(error) => Some((Err(error), (scoped, true))),
            }
        });
        ConnectionsStream {
            inner: Box::pin(inner),
        }
    }
}

/// One main-stream frame as the run's response frame: a connector
/// come or gone is an item, the provider's error is the end, and the
/// id or a refusal — forbidden after the first response — is read
/// past.
fn decode(payload: &[u8]) -> Result<Decoded<Connection>, response::FrameError> {
    Ok(match response::Frame::decode(payload)? {
        response::Frame::Connected(connector) => Decoded::Item(Connection::Connected(connector)),
        response::Frame::Disconnected(connector) => Decoded::Item(Connection::Disconnected(connector)),
        response::Frame::Error(error) => Decoded::Error(error),
        response::Frame::Id(_) | response::Frame::VolumeHeld(_) | response::Frame::VolumeMode(_) => Decoded::Skip,
    })
}

impl Stream for ConnectionsStream {
    type Item = Result<Connection, WaitError<response::FrameError>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl fmt::Debug for ConnectionsStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConnectionsStream")
    }
}
