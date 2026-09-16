//! What a container scope supplies to the shared machinery.
//!
//! The three scopes' frames are byte-identical and distinct types,
//! and their containers begin differently, so the machinery is
//! generic over a [`Family`] — every scope — and over [`Runs`] — the
//! two that deploy — and each scope's `handle` names its own.

use std::future::Future;
use std::sync::Arc;

use bytes::Bytes;
use serde_json::Value;

use super::begin::Begun;
use super::own::Own;
use super::run::Run;
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::Ask;
use crate::container_proxy_endpoints::fuse::mount::client::execute::Ask as MountAsk;
use crate::decode::Decode;
use crate::encode::Encode;
use crate::shared::containers::response::{Id, VolumeMounted};
use crate::shared::error::Error;
use crate::shared::filetree;

/// What every container scope supplies: its frames.
///
/// Each method encodes one frame of this family's own types, or
/// decodes one, so the machinery can speak them without knowing which
/// scope it serves. Every encoder answers [`None`] when the frame
/// would not encode, which the machinery treats as an answer it
/// cannot give.
pub(crate) trait Family: Send + Sync + 'static {
    /// The caller's channel request frame.
    type Request: for<'a> Decode<'a> + Send;
    /// The family's own exchanges, past the shared six.
    type Exchange: Send + 'static;

    /// Which channel the caller opened.
    fn classify(request: Self::Request) -> Opened<Self::Exchange>;
    /// Serve one of the family's own exchanges, to the end.
    fn serve(run: Arc<Run>, channel: u32, exchange: Self::Exchange) -> impl Future<Output = ()> + Send + 'static;

    /// The scope's error, on channel `0`.
    fn error(error: &Error) -> Option<Vec<u8>>;
    /// The ask for a write's content, quoting the caller's write id.
    fn write_ask(write_id: u32) -> Option<Vec<u8>>;
    /// One piece of a write's content, or the caller's error.
    fn content(payload: &Bytes) -> Result<Bytes, Error>;

    /// One filetree frame, as this family's channel response.
    fn filetree(frame: filetree::response::Frame) -> Option<Vec<u8>>;
    /// A filetree channel's error.
    fn filetree_error(error: &Error) -> Option<Vec<u8>>;
    /// One piece of a read.
    fn read_body(bytes: &[u8]) -> Option<Vec<u8>>;
    /// A read channel's error.
    fn read_error(error: &Error) -> Option<Vec<u8>>;
    /// A write that landed.
    fn written() -> Option<Vec<u8>>;
    /// A write channel's error.
    fn write_error(error: &Error) -> Option<Vec<u8>>;
    /// A transfer that landed.
    fn transferred() -> Option<Vec<u8>>;
    /// A transfer channel's error.
    fn transfer_error(error: &Error) -> Option<Vec<u8>>;
}

/// What the two scopes that deploy supply besides: how their container
/// begins, and how the proxy's asks are put to the caller.
pub(crate) trait Runs: Family {
    /// The channel request frame this end opens on the caller.
    type Ask<'a>: Encode + From<Own<'a>>;

    /// Begin the container's proxy: the family's begin scope on the
    /// connection `proxy`, carrying `agent` where the family takes
    /// one. What comes back is the scope and what rides it; an error
    /// is the run's, in the proxy's words where it refused.
    fn begin(proxy: &Handle, agent: Option<Value>) -> impl Future<Output = Result<Begun, Error>> + Send;

    /// The proxy's ask on the begin scope, as this family's frame to
    /// the caller — or [`None`] for the one that is not carried as it
    /// came: a database connection, which this end re-asks under an
    /// id of its own. See [`Own::Postgres`].
    fn relayed<'a>(ask: &'a Ask) -> Option<Self::Ask<'a>>;

    /// A mount's ask, as this family's frame to the caller, with the
    /// caller's id for the mount put back in front of it.
    fn fuse<'a>(mount_id: &'a str, ask: &'a MountAsk) -> Self::Ask<'a>;

    /// The container's id, on channel `0`.
    fn id(id: &Id) -> Option<Vec<u8>>;
    /// The run refused for a held volume, on channel `0`.
    fn volume_mounted(refused: &VolumeMounted) -> Option<Vec<u8>>;
}

/// What a caller opened, classified.
#[derive(Debug)]
pub(crate) enum Opened<E> {
    /// Stop the container, or leave it.
    Stop,
    /// The container's tree, watched.
    Filetree,
    /// One file, read.
    Read(Vec<String>),
    /// One file, written.
    Write {
        /// The caller's id for the write, quoted on the content ask.
        write_id: u32,
        /// The destination.
        path: Vec<String>,
    },
    /// One file, copied into another container.
    Transfer {
        /// The file, in this run's container.
        path: Vec<String>,
        /// The other container, by its id.
        id: String,
        /// The destination, in that container.
        destination: Vec<String>,
    },
    /// The caller's half of a database connection, by the id this end
    /// minted.
    Postgres(u32),
    /// The family's own.
    Exchange(E),
}
