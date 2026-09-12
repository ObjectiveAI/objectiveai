//! A running container scope, as every `ExecuteHandle` holds it.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Answered, ChannelStream, UnaryError, Writes, unary};
use crate::client::handle::{Handle, SendError};
use crate::frame;
use crate::shared::error::Error;

/// The scope a run or a connection opened, held for its life: the
/// handle and number every channel is opened on, the writes started
/// and not yet asked for, and the main stream's end.
///
/// Behind an [`Arc`] in each family's `ExecuteHandle`, so a clone of
/// the handle is a clone of the scope's whole state, and every clone
/// sees the same end.
pub struct Scoped {
    handle: Handle,
    scope: u32,
    writes: Arc<Writes>,
    main: Mutex<Main>,
}

/// The main stream: still open, or ended one way or the other.
enum Main {
    /// Frames may still come.
    Open(UnboundedReceiver<Bytes>),
    /// The finish came: the run is over, or the connection.
    Finished,
    /// An error came, then the finish.
    Failed(Error),
}

impl Scoped {
    pub(crate) fn new(handle: Handle, scope: u32, writes: Arc<Writes>, main: UnboundedReceiver<Bytes>) -> Self {
        Scoped {
            handle,
            scope,
            writes,
            main: Mutex::new(Main::Open(main)),
        }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Open a channel that streams, with `payload` as its request.
    pub(crate) async fn open<A: Answered>(&self, payload: &[u8]) -> Result<ChannelStream<A>, SendError> {
        let channel = self.handle.send_channel_request(self.scope, payload).await?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// Open a channel that answers once, and take the answer.
    pub(crate) async fn unary<A: Answered>(&self, payload: &[u8]) -> Result<A::Item, UnaryError<A>> {
        unary::<A>(&self.handle, self.scope, payload).await
    }

    /// Open a channel nothing answers on — a stop, a disconnect — and
    /// let it go.
    pub(crate) async fn bare(&self, payload: &[u8]) -> Result<(), SendError> {
        self.handle
            .send_channel_request(self.scope, payload)
            .await
            .map(|_| ())
    }

    /// Write a file: keep `content` under a fresh write id, open the
    /// `write_path` channel `encode` builds for that id, and take its
    /// answer. The provider asks for the content on a channel of its
    /// own, which the serving loop answers from the pending writes. A
    /// request that never goes out takes the content back out of the
    /// pending set.
    pub(crate) async fn write<A, S, E>(
        &self,
        encode: impl FnOnce(u32) -> Result<Vec<u8>, serde_json::Error>,
        content: S,
    ) -> Result<(), UnaryError<A>>
    where
        A: Answered<Item = ()>,
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: fmt::Display + Send + 'static,
    {
        let content = content.map(|piece| piece.map_err(|error| Error(serde_json::Value::String(error.to_string()))));
        let write_id = self.writes.register(Box::pin(content));
        let payload = match encode(write_id) {
            Ok(payload) => payload,
            Err(error) => {
                self.writes.take(write_id);
                return Err(UnaryError::Request(error));
            }
        };
        let result = self.unary::<A>(&payload).await;
        if matches!(result, Err(UnaryError::Send(_))) {
            self.writes.take(write_id);
        }
        result
    }

    /// The scope's end.
    ///
    /// Resolves when the main stream finishes: `Ok` for the run over
    /// or the connection closed as it should, `Err` for the provider's
    /// own error frame, the connection dying first, or a frame that
    /// should not be there. `decode` reads one main-stream payload as
    /// the family's response frame: `Ok(None)` for a frame that says
    /// nothing about the end (a run's id, again), `Ok(Some(error))`
    /// for the provider's error. An end once known is answered again
    /// the same way to every later call.
    pub(crate) async fn wait<R, D>(&self, decode: D) -> Result<(), WaitError<R>>
    where
        D: Fn(&[u8]) -> Result<Option<Error>, R>,
    {
        let mut main = self.main.lock().await;
        loop {
            let receiver = match &mut *main {
                Main::Open(receiver) => receiver,
                Main::Finished => return Ok(()),
                Main::Failed(error) => return Err(WaitError::Provider(error.clone())),
            };
            let Some(bytes) = receiver.recv().await else {
                *main = Main::Finished;
                return Err(WaitError::Closed);
            };
            let envelope = match frame::server::ServerFrame::decode(&bytes) {
                Ok(envelope) => envelope,
                Err(error) => {
                    *main = Main::Finished;
                    return Err(WaitError::Frame(error));
                }
            };
            match envelope {
                frame::server::ServerFrame::Response { payload, .. } => match decode(payload) {
                    Ok(None) => continue,
                    Ok(Some(error)) => {
                        *main = Main::Failed(error.clone());
                        return Err(WaitError::Provider(error));
                    }
                    Err(error) => {
                        *main = Main::Finished;
                        return Err(WaitError::Response(error));
                    }
                },
                frame::server::ServerFrame::ResponseFinish { .. } => {
                    *main = Main::Finished;
                    return Ok(());
                }
                _ => {
                    *main = Main::Finished;
                    return Err(WaitError::Misrouted);
                }
            }
        }
    }
}

impl fmt::Debug for Scoped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scoped")
            .field("scope", &self.scope)
            .field("writes", &self.writes)
            .finish_non_exhaustive()
    }
}

/// A scope that stopped without ending as it should.
#[derive(Debug)]
pub enum WaitError<R> {
    /// The connection ended before the scope's finish.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on the main stream.
    Misrouted,
    /// The main-stream frame did not parse.
    Response(R),
    /// The provider ended the scope with its own error: the container
    /// never came up, or is gone; the connection never was, or its
    /// runner said no.
    Provider(Error),
}

impl<R: fmt::Display> fmt::Display for WaitError<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaitError::Closed => f.write_str("connection ended before the scope finished"),
            WaitError::Frame(error) => write!(f, "main stream frame did not decode: {error}"),
            WaitError::Misrouted => f.write_str("a frame arrived that does not belong on the main stream"),
            WaitError::Response(error) => write!(f, "main stream frame did not parse: {error}"),
            WaitError::Provider(_) => f.write_str("the provider ended the scope with an error"),
        }
    }
}

impl<R: std::error::Error + 'static> std::error::Error for WaitError<R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WaitError::Frame(error) => Some(error),
            WaitError::Response(error) => Some(error),
            WaitError::Closed | WaitError::Misrouted | WaitError::Provider(_) => None,
        }
    }
}
