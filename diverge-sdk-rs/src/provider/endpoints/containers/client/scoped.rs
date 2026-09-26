//! A running container scope, as every `ExecuteHandle` holds it.

use std::convert::Infallible;
use std::fmt;
use std::pin::pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{Mutex, Notify};

use super::{Answered, ChannelStream, UnaryError, Writes, unary};
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::frame;
use crate::shared::error::Error;

/// The scope a run or a connection opened, held for its life: the
/// handle and number every channel is opened on, the writes started
/// and not yet asked for, and the main stream — read by exactly one
/// reader, its ending kept for everyone.
///
/// Behind an [`Arc`] in each family's `ExecuteHandle`, so a clone of
/// the handle is a clone of the scope's whole state, and every clone
/// sees the same end.
///
/// # The main stream has one reader
///
/// `read` takes frames off it under a lock, one at a
/// time, decoding each with the family's own reading of its response
/// frame: an item to hand back, a frame that says nothing, or the
/// provider's error. Whatever ends the stream — the finish, the
/// connection, the provider's error, a frame that cannot be there —
/// is kept as an [`Ending`] and answered again, the same way, to
/// every later reader and to every `ended`. Nothing a
/// provider sends on the main stream is read and thrown away: a
/// family whose main stream carries items hands them out as a
/// stream; a family whose main stream carries nothing after the
/// first response reads it only to hear the end.
pub struct Scoped {
    handle: Handle,
    scope: u32,
    writes: Arc<Writes>,
    main: Mutex<Main>,
    /// Fired when the main stream ends, however it ends, for
    /// `ended`.
    ended: Notify,
}

/// The main stream: still open, or ended one way or another.
enum Main {
    /// Frames may still come.
    Open(UnboundedReceiver<Bytes>),
    /// Nothing more will: this is how it ended.
    Ended(Ending),
}

/// How a main stream ended, kept so that every later reader is told
/// the same thing.
///
/// Every reason is [`Clone`], which is what makes it replayable — the
/// one that is not, a response decode error, is kept as its message.
#[derive(Debug, Clone)]
pub enum Ending {
    /// The finish came: the run is over, or the connection.
    Finished,
    /// The connection ended before the finish.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on the main stream.
    Misrouted,
    /// A main-stream frame did not parse, and this is what the
    /// parser said.
    Undecodable(String),
    /// The provider ended the scope with its own error.
    Provider(Error),
}

/// What one main-stream frame decodes to, for `Scoped::read`.
#[derive(Debug)]
pub enum Decoded<T> {
    /// Something to hand back.
    Item(T),
    /// The provider's error: the scope is over.
    Error(Error),
    /// A frame that says nothing about the stream — a wire-forbidden
    /// repeat of a first response, on the families that have one.
    Skip,
}

impl Scoped {
    pub(crate) fn new(handle: Handle, scope: u32, writes: Arc<Writes>, main: UnboundedReceiver<Bytes>) -> Self {
        Scoped {
            handle,
            scope,
            writes,
            main: Mutex::new(Main::Open(main)),
            ended: Notify::new(),
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
    /// write that fails for any reason takes the content back out of
    /// the pending set — a request that never went out, or an answer
    /// that did not come — so nothing is kept for an ask that will
    /// not arrive.
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
        if result.is_err() {
            self.writes.take(write_id);
        }
        result
    }

    /// One frame off the main stream, decoded.
    ///
    /// `decode` reads one main-stream payload as the family's
    /// response frame. [`Item`](Decoded::Item) is handed back;
    /// [`Skip`](Decoded::Skip) is read past; [`Error`](Decoded::Error) is
    /// the provider ending the scope, kept and returned. [`Ok`]`(None)`
    /// is the finish — the run over, or the connection, as it should
    /// — and [`Err`] everything else that ends a stream. A stream that
    /// has ended answers every later call the way it answered the
    /// first.
    pub(crate) async fn read<T, R, D>(&self, decode: D) -> Result<Option<T>, WaitError<R>>
    where
        D: Fn(&[u8]) -> Result<Decoded<T>, R>,
        R: fmt::Display,
    {
        let mut main = self.main.lock().await;
        loop {
            let receiver = match &mut *main {
                Main::Open(receiver) => receiver,
                Main::Ended(ending) => return replay(ending),
            };
            let Some(bytes) = receiver.recv().await else {
                return self.end(&mut main, Ending::Closed);
            };
            let envelope = match frame::server::ServerFrame::decode(&bytes) {
                Ok(envelope) => envelope,
                Err(error) => return self.end(&mut main, Ending::Frame(error)),
            };
            match envelope {
                frame::server::ServerFrame::Response { payload, .. } => match decode(payload) {
                    Ok(Decoded::Item(item)) => return Ok(Some(item)),
                    Ok(Decoded::Skip) => continue,
                    Ok(Decoded::Error(error)) => return self.end(&mut main, Ending::Provider(error)),
                    Err(error) => {
                        // The first reader gets the parser's own
                        // error; every later one gets its message.
                        *main = Main::Ended(Ending::Undecodable(error.to_string()));
                        self.ended.notify_waiters();
                        return Err(WaitError::Response(error));
                    }
                },
                frame::server::ServerFrame::ResponseFinish { .. } => {
                    return self.end(&mut main, Ending::Finished);
                }
                _ => return self.end(&mut main, Ending::Misrouted),
            }
        }
    }

    /// The scope's end, for a family whose main stream carries nothing
    /// after its first response.
    ///
    /// Resolves when the main stream finishes: `Ok` for the run over
    /// or the connection closed as it should, `Err` for the provider's
    /// own error frame, the connection dying first, or a frame that
    /// should not be there. `decode` never yields an item, so this is
    /// `read` to the end. An end once known is answered
    /// again the same way to every later call.
    pub(crate) async fn wait<R, D>(&self, decode: D) -> Result<(), WaitError<R>>
    where
        D: Fn(&[u8]) -> Result<Decoded<Infallible>, R>,
        R: fmt::Display,
    {
        match self.read(decode).await {
            Ok(Some(never)) => match never {},
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// The scope's end, without reading the main stream: for a family
    /// whose main stream carries items, which somebody else is
    /// reading.
    ///
    /// Resolves when whoever reads the stream reads its end, and
    /// answers the same way to every call after. It reads nothing
    /// itself, so that no item is lost to a caller who only wanted
    /// the end: a caller that wants the end and not the items drains
    /// the stream.
    pub(crate) async fn ended<R>(&self) -> Result<(), WaitError<R>> {
        loop {
            // Registered before the lock is taken, so a latch between
            // the check and the wait is not missed.
            let mut notified = pin!(self.ended.notified());
            notified.as_mut().enable();
            {
                let main = self.main.lock().await;
                if let Main::Ended(ending) = &*main {
                    return replay::<Infallible, R>(ending).map(|_| ());
                }
            }
            notified.await;
        }
    }

    /// Keep the ending, wake every `ended`, and answer
    /// this reader with it.
    fn end<T, R>(&self, main: &mut Main, ending: Ending) -> Result<Option<T>, WaitError<R>> {
        let result = replay(&ending);
        *main = Main::Ended(ending);
        self.ended.notify_waiters();
        result
    }
}

/// An ending, as a reader's answer.
fn replay<T, R>(ending: &Ending) -> Result<Option<T>, WaitError<R>> {
    match ending {
        Ending::Finished => Ok(None),
        Ending::Closed => Err(WaitError::Closed),
        Ending::Frame(error) => Err(WaitError::Frame(*error)),
        Ending::Misrouted => Err(WaitError::Misrouted),
        Ending::Undecodable(message) => Err(WaitError::Undecodable(message.clone())),
        Ending::Provider(error) => Err(WaitError::Provider(error.clone())),
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
    /// The main-stream frame did not parse, and an earlier reader was
    /// told so: this is what the parser said, replayed.
    Undecodable(String),
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
            WaitError::Undecodable(message) => write!(f, "main stream frame did not parse: {message}"),
            WaitError::Provider(_) => f.write_str("the provider ended the scope with an error"),
        }
    }
}

impl<R: std::error::Error + 'static> std::error::Error for WaitError<R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WaitError::Frame(error) => Some(error),
            WaitError::Response(error) => Some(error),
            WaitError::Closed | WaitError::Misrouted | WaitError::Undecodable(_) | WaitError::Provider(_) => None,
        }
    }
}
