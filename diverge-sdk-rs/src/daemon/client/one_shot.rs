//! One question, one answer, and the scope is over.

use std::fmt;

use crate::wire::client::handle::{Handle, SendError};
use crate::wire::client::scope::Scope;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Send one request and read its one answer.
///
/// Opening a scope, writing the request, waiting for the one frame
/// that answers it, and taking that answer out of two layers of
/// envelope are the same steps for every one-answer endpoint of the
/// daemon's, and there is nothing in them a caller gets to decide. The
/// answer comes back whole: whichever variant the daemon chose,
/// its own `Error` included.
///
/// # It reads one frame and leaves
///
/// The scope's request receiver is dropped unread: the daemon opens no
/// channel on a one-answer scope. The finish that follows the answer
/// is never read; dropping the [`Scope`] is what tells the router
/// nobody is listening for the rest.
///
/// # What ends the wait
///
/// One frame, and there is no timeout. A daemon that never answers is
/// waited on until the connection dies, at which point the receiver
/// closes and this returns [`Error::Closed`].
pub async fn execute<Q, A, E>(handle: &Handle, request: &Q) -> Result<A, Error<Q::Error, E>>
where
    Q: Encode,
    A: for<'a> Decode<'a, Error = E>,
{
    let mut scope = open::<Q, E>(handle, request).await?;
    let bytes = scope.response_receiver.recv().await.ok_or(Error::Closed)?;
    answer(&bytes)
}

/// Encode the request and open its scope.
pub(crate) async fn open<Q: Encode, E>(handle: &Handle, request: &Q) -> Result<Scope, Error<Q::Error, E>> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(Error::Request)?;
    handle.send_request(&payload).await.map_err(Error::Send)
}

/// One frame off a scope's response receiver, read as the answer.
pub(crate) fn answer<Q, A, E>(bytes: &[u8]) -> Result<A, Error<Q, E>>
where
    A: for<'a> Decode<'a, Error = E>,
{
    let envelope = frame::server::ServerFrame::decode(bytes).map_err(Error::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(Error::Unanswered),
        _ => return Err(Error::Misrouted),
    };
    A::decode(payload).map_err(Error::Response)
}

/// A one-answer exchange that produced no answer.
///
/// Every one is this end's view of the machinery failing. None is the
/// daemon refusing: a refusal is an answer, and comes back as the
/// response frame.
#[derive(Debug)]
pub enum Error<Q, A> {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request did not serialize.
    Request(Q),
    /// The connection ended before an answer arrived.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The daemon finished the scope without answering: a request it
    /// could not read, as [`endpoints`](crate::daemon::endpoints)
    /// states.
    Unanswered,
    /// A frame that cannot be the first on a main stream.
    Misrouted,
    /// The answer did not parse.
    Response(A),
}

impl<Q: fmt::Display, A: fmt::Display> fmt::Display for Error<Q, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Send(error) => write!(f, "the request never went out: {error}"),
            Error::Request(error) => write!(f, "the request did not serialize: {error}"),
            Error::Closed => f.write_str("the connection ended before the daemon answered"),
            Error::Frame(error) => write!(f, "the answer did not decode: {error}"),
            Error::Unanswered => f.write_str("the daemon finished the scope without an answer"),
            Error::Misrouted => f.write_str("a frame that cannot open an answer arrived"),
            Error::Response(error) => write!(f, "the answer did not parse: {error}"),
        }
    }
}

impl<Q, A> std::error::Error for Error<Q, A>
where
    Q: std::error::Error + 'static,
    A: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Send(error) => Some(error),
            Error::Request(error) => Some(error),
            Error::Frame(error) => Some(error),
            Error::Response(error) => Some(error),
            Error::Closed | Error::Unanswered | Error::Misrouted => None,
        }
    }
}
