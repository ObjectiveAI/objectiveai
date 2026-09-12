//! What a family says about its own frames.

use std::future::Future;
use std::sync::Arc;

use bytes::Bytes;

use super::own::Own;
use super::run::Run;
use crate::container_proxy::requests::request::Request;
use crate::decode::Decode;
use crate::encode::Encode;
use crate::shared::containers::response::{Id, VolumeMounted};
use crate::shared::error::Error;
use crate::shared::filetree;

/// One container scope's frames, named once.
///
/// The three scopes carry the same exchanges under types of their
/// own: a filetree answer is `agents::run`'s `filetree::Frame` or
/// `tools::run`'s or `tools::connect`'s, byte-identical and distinct.
/// The machinery is written against this, and a handler's `family`
/// module says which types those are — every method here is a
/// construction or a decode of one of them, and nothing else.
///
/// Encoders answer `None` for a frame that would not serialize, which
/// the machinery sends as nothing; a decode that fails is the error
/// the exchange's own vocabulary has for it.
pub(crate) trait Family: Send + Sync + 'static {
    /// The channel request the caller opens: the family's
    /// `client::channel_request::Frame`.
    type Request: for<'a> Decode<'a> + Send;

    /// The family's own exchanges, once the shared five are taken off.
    type Exchange: Send + 'static;

    /// Read one opened channel as what it asks.
    fn classify(request: Self::Request) -> Opened<Self::Exchange>;

    /// Serve one of the family's own exchanges on `channel`, to the
    /// end: every response, then the finish.
    fn serve(run: Arc<Run>, channel: u32, exchange: Self::Exchange) -> impl Future<Output = ()> + Send + 'static;

    /// The main stream's error: the scope ending badly.
    fn error(error: &Error) -> Option<Vec<u8>>;

    /// The one ask every family makes: the content of a write the
    /// caller started, by the id it gave.
    fn write_ask(write_id: u32) -> Option<Vec<u8>>;

    /// One answer on that channel: a piece of the content, or the
    /// caller saying it stopped.
    fn content(payload: &Bytes) -> Result<Bytes, Error>;

    /// A filetree answer: one frame of the tree.
    fn filetree(frame: filetree::response::Frame) -> Option<Vec<u8>>;
    /// A filetree answer: the tree cannot be had.
    fn filetree_error(error: &Error) -> Option<Vec<u8>>;
    /// A read answer: a piece of the file.
    fn read_body(bytes: &[u8]) -> Option<Vec<u8>>;
    /// A read answer: the file cannot be had.
    fn read_error(error: &Error) -> Option<Vec<u8>>;
    /// A write answer: the file landed.
    fn written() -> Option<Vec<u8>>;
    /// A write answer: it did not.
    fn write_error(error: &Error) -> Option<Vec<u8>>;
}

/// What a family that RUNS a container adds: the asks it makes of the
/// caller, its own and the container's relayed, and the id it
/// answers with.
pub(crate) trait Runs: Family {
    /// The channel request this end opens: the family's
    /// `server::channel_request::Frame`, with every provider's-own ask
    /// spelled in it.
    type Ask<'a>: Encode + From<Own<'a>>;

    /// A container's ask as this family's frame, to carry to the
    /// caller. `None` for the one that is not carried as it came:
    /// a database connection, which this end re-asks under an id of
    /// its own — see [`Own::Postgres`].
    fn relayed<'a>(request: Request<'a>) -> Option<Self::Ask<'a>>;

    /// The main stream's first and only good word: the container's id.
    fn id(id: &Id) -> Option<Vec<u8>>;

    /// The refusal that is not an error: a volume the request names
    /// is mounted in another container of the caller's.
    fn volume_mounted(refused: &VolumeMounted) -> Option<Vec<u8>>;
}

/// A channel the caller opened, read as what it asks.
#[derive(Debug)]
pub(crate) enum Opened<E> {
    /// Stop the container, or leave it: the scope is over.
    Stop,
    /// The container's filesystem, watched.
    Filetree,
    /// One file out of the container.
    Read(Vec<String>),
    /// One file into it: the content follows on a channel this end
    /// opens for `write_id`.
    Write { write_id: u32, path: Vec<String> },
    /// The caller's half of a database connection this end asked for.
    Postgres(u32),
    /// The family's own.
    Exchange(E),
}
