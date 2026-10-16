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
use super::own::{Own, Unasked};
use super::run::Run;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::container_proxy::outside::client::Ask;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute::Ask as MountAsk;
use diverge_sdk::wire::decode::Decode;
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::shared::containers::response::{
    Id, VolumeFuseServed, VolumeHeld, VolumeMode, VolumeMountUnsupported, VolumeMounted,
};
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::fuse_mount;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree;

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
    /// The family's own exchanges, past the shared seven.
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
    /// A mount asked for while the container runs: made, or why not.
    fn fuse_mount(frame: &fuse_mount::response::Frame) -> Option<Vec<u8>>;
}

/// What the two scopes that deploy supply besides: how their container
/// begins, and how the proxy's asks are put to the caller.
pub(crate) trait Runs: Family {
    /// The channel request frame this end opens on the caller: every
    /// [`Own`] as the family spells it, or [`Unasked`] for the one the
    /// family's wire has no frame for.
    type Ask<'a>: Encode + TryFrom<Own<'a>, Error = Unasked>;

    /// Begin the container's proxy: the family's begin scope on the
    /// connection `proxy`, carrying the container's `arguments` and
    /// `template`, the id of the template it is made from.
    /// What comes back is the scope and what rides it; an
    /// error is the run's, in the proxy's words where it refused.
    fn begin(proxy: &Handle, arguments: Value, template: String) -> impl Future<Output = Result<Begun, Error>> + Send;

    /// The dependencies the container declared, asked of the caller
    /// before the id goes out but with the id minted and told: an
    /// agent container's on its `dependencies` channel, nothing when
    /// it declared none; a tool container declares nothing and
    /// nothing is asked. A deploy is `Ok`; anything else is the run's
    /// error, in the caller's words where it refused.
    fn deploy(scope: &ScopeHandle, id: &str, declared: &[Template]) -> impl Future<Output = Result<(), Error>> + Send;

    /// The proxy's ask on the begin scope, as this family's frame to
    /// the caller — or [`None`] for one that is not carried as it
    /// came: a database connection or a daemon connection, which this
    /// end re-asks under an id of its own, see [`Own`]; and one the
    /// family's wire has no frame for, which a proxy of that family
    /// never opens.
    fn relayed<'a>(ask: &'a Ask) -> Option<Self::Ask<'a>>;

    /// A mount's ask, as this family's frame to the caller, with the
    /// caller's id for the mount put back in front of it.
    fn fuse<'a>(mount_id: &'a str, ask: &'a MountAsk) -> Self::Ask<'a>;

    /// The container's id, on channel `0`.
    fn id(id: &Id) -> Option<Vec<u8>>;
    /// The run refused for a volume an operation has, on channel `0`.
    fn volume_held(refused: &VolumeHeld) -> Option<Vec<u8>>;
    /// The run refused for a volume a running container has, on
    /// channel `0`.
    fn volume_mounted(refused: &VolumeMounted) -> Option<Vec<u8>>;
    /// The run refused for a volume a FUSE serve has, on channel `0`.
    fn volume_fuse_served(refused: &VolumeFuseServed) -> Option<Vec<u8>>;
    /// The run refused for a volume of a mode the provider does not
    /// mount, on channel `0`.
    fn volume_mount_unsupported(refused: &VolumeMountUnsupported) -> Option<Vec<u8>>;
    /// The run refused for a volume in another mode, on channel `0`.
    fn volume_mode(refused: &VolumeMode) -> Option<Vec<u8>>;
}

/// What a caller opened, classified.
#[derive(Debug)]
pub(crate) enum Opened<E> {
    /// Stop the container, or leave it.
    Stop,
    /// The container's tree, watched.
    Filetree,
    /// A daemon connection the proxy announced, by an id of this
    /// end's own: a tool container's alone.
    Daemon(u32),
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
    /// What the arguments may be.
    Schema,
    /// One more FUSE mount, made while the container runs.
    FuseMount(fuse_mount::request::Request),
    /// The family's own.
    Exchange(E),
}
