//! A running agent container, and everything a caller can do with
//! it.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::BoxFuture;
use futures_util::{FutureExt as _, Stream};
use rmcp::model::ContentBlock;

use super::super::channel_request;
use super::{Exec, ExecStream, Filetree, FiletreeStream, Read, ReadStream, Transfer, WritePath};
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::agents::run::server;
use crate::provider::endpoints::containers::client::answered::{Enqueue, FuseMount, Info, Secrets};
use crate::provider::endpoints::containers::client::{ChannelStream, OpenError, Scoped, UnaryError, WaitError, unary_answer};
use crate::shared::containers::fuse::Kind;
use crate::shared::containers::exec;
use crate::shared::containers::terminal;
use crate::provider::endpoints::containers::client::answered::{FrontendOutput, TerminalDone, TerminalOpened, TerminalOutput};
use crate::shared::containers::frontend;
use crate::shared::containers::info::Document;
use crate::shared::containers::secrets::Values;
use crate::shared::containers::{enqueue, fuse_mount, read, transfer, write_path};

/// The scope a run opened, held for the container's life.
///
/// Every channel a caller may open into an agent container is a
/// method here: the tree watched, a file read or written, one more
/// mount made, the image's DIVERGE.md, the queue's two verbs, the
/// secrets' feed, and the cancel. What the agent says
/// is not: it is the [`ExecuteStream`](super::ExecuteStream) `execute` handed back
/// beside this. Each opens its own channel, so several may be in flight
/// at once. Clones share the scope, and [`wait`](Self::wait) on any
/// of them reports the same end.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame: a destructor could not await, could
/// not report a failure, and could not be skipped when a caller
/// wanted the container to outlive the value. Ending a run is
/// [`cancel`](Self::cancel), or the connection going.
#[derive(Debug, Clone)]
pub struct ExecuteHandle(Arc<Scoped>);

/// One channel request of this family, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        ExecuteHandle(scoped)
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.0.scope()
    }

    /// The run's end: `Ok` when the scope finishes as it should — a
    /// cancel, or the container's own end — and the provider's error
    /// when it ended that way. Resolves once and answers the same way
    /// again after.
    ///
    /// # It reads nothing
    ///
    /// The main stream is the conversation, and the
    /// [`ExecuteStream`](super::ExecuteStream) `execute` handed back is its reader.
    /// This waits for that reader to reach the end, so that no chunk
    /// is lost to a caller who only wanted to know the run is over; a
    /// caller that wants the end and not the conversation drains the
    /// chunks.
    pub async fn wait(&self) -> Result<(), WaitError<server::response::FrameError>> {
        self.0.ended().await
    }

    /// Cancel the run: stop the container. Nothing answers on the channel this opens;
    /// what answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn cancel(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Cancel).map_err(OpenError::Request)?;
        self.0.bare(&payload).await.map_err(OpenError::Send)
    }

    /// Watch the container's tree: a snapshot, then every change.
    pub async fn filetree(&self) -> Result<FiletreeStream, OpenError> {
        let payload = payload(&channel_request::Frame::Filetree).map_err(OpenError::Request)?;
        self.0.open::<Filetree>(&payload).await.map_err(OpenError::Send)
    }

    /// Read one file out of the container, as path components from
    /// its root.
    pub async fn read(&self, path: Vec<String>) -> Result<ReadStream, OpenError> {
        let payload = payload(&channel_request::Frame::Read(read::request::Request { path }))
            .map_err(OpenError::Request)?;
        self.0.open::<Read>(&payload).await.map_err(OpenError::Send)
    }

    /// Run one command in the container, its output streamed (see
    /// [`exec`](crate::shared::containers::exec)). Dropping the stream
    /// abandons it, which kills the command.
    pub async fn exec(&self, command: exec::request::Request) -> Result<ExecStream, OpenError> {
        let payload = payload(&channel_request::Frame::Exec(command)).map_err(OpenError::Request)?;
        self.0.open::<Exec>(&payload).await.map_err(OpenError::Send)
    }

    /// Start a terminal the provider holds, and answer its id (see
    /// [`session`](crate::shared::containers::terminal::session)).
    pub async fn terminal_open(&self, size: terminal::open::Open) -> Result<u32, UnaryError<TerminalOpened>> {
        let payload = payload(&channel_request::Frame::TerminalOpen(size)).map_err(UnaryError::Request)?;
        self.0.unary::<TerminalOpened>(&payload).await
    }

    /// Stream a held terminal's output: its scrollback, then live.
    /// Dropping the stream detaches; the terminal goes on.
    pub async fn terminal_attach(&self, id: u32) -> Result<ChannelStream<TerminalOutput>, OpenError> {
        let payload = payload(&channel_request::Frame::TerminalAttach(terminal::session::Id { id })).map_err(OpenError::Request)?;
        self.0.open::<TerminalOutput>(&payload).await.map_err(OpenError::Send)
    }

    /// Write bytes to a held terminal's stdin; resolves once written.
    pub async fn terminal_stdin(&self, id: u32, bytes: Vec<u8>) -> Result<(), UnaryError<TerminalDone>> {
        let payload = payload(&channel_request::Frame::TerminalStdin { id, bytes }).map_err(UnaryError::Request)?;
        self.0.unary::<TerminalDone>(&payload).await
    }

    /// Set a held terminal's size.
    pub async fn terminal_resize(&self, id: u32, cols: u16, rows: u16) -> Result<(), UnaryError<TerminalDone>> {
        let payload = payload(&channel_request::Frame::TerminalResize(terminal::session::Resize { id, cols, rows })).map_err(UnaryError::Request)?;
        self.0.unary::<TerminalDone>(&payload).await
    }

    /// Kill a held terminal's shell.
    pub async fn terminal_close(&self, id: u32) -> Result<(), UnaryError<TerminalDone>> {
        let payload = payload(&channel_request::Frame::TerminalClose(terminal::session::Id { id })).map_err(UnaryError::Request)?;
        self.0.unary::<TerminalDone>(&payload).await
    }

    /// One HTTP request to the program's frontend: `head`, its body
    /// `body`'s pieces, which the provider asks for; its answer as a
    /// stream, and the write id the body is kept under until
    /// [`forget`](Self::forget) takes it back. Dropping the stream
    /// abandons the request.
    pub async fn frontend<S, E>(&self, head: frontend::request::Head, body: S) -> Result<(ChannelStream<FrontendOutput>, u32), OpenError>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: fmt::Display + Send + 'static,
    {
        self.0
            .open_writing::<FrontendOutput, S, E>(move |write_id| payload(&channel_request::Frame::Frontend(frontend::request::Request { write_id, head })), body)
            .await
    }

    /// Take back a body kept for a request whose provider never asked
    /// for it: called once the request's stream is done.
    pub async fn forget(&self, write_id: u32) {
        self.0.forget(write_id).await;
    }

    /// Write one file into the container, whole, from `content`'s
    /// pieces; resolves when it is at the path. A piece that fails
    /// ends the content with an error the provider sees, and the
    /// provider's own answer is what comes back.
    pub async fn write<S, E>(&self, path: Vec<String>, content: S) -> Result<(), UnaryError<WritePath>>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: fmt::Display + Send + 'static,
    {
        self.0
            .write::<WritePath, S, E>(
                move |write_id| {
                    payload(&channel_request::Frame::Write(write_path::request::Request {
                        write_id,
                        path,
                    }))
                },
                content,
            )
            .await
    }

    /// Copy one file out of this container into the container under
    /// `id`, at `destination`, without the bytes passing through here;
    /// resolves when it is at the destination. The caller must be
    /// running, or connected to, both containers, or the provider
    /// refuses.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), UnaryError<Transfer>> {
        let payload = payload(&channel_request::Frame::Transfer(transfer::request::Request { path, id, destination }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<Transfer>(&payload).await
    }

    /// What the arguments may be: the image's `/DIVERGE.md`, parsed —
    /// its name, its schema and its body — or `None` for an image that ships
    /// none.
    pub async fn info(&self) -> Result<Option<Document>, UnaryError<Info>> {
        let payload = payload(&channel_request::Frame::Info).map_err(UnaryError::Request)?;
        self.0.unary::<Info>(&payload).await
    }

    /// A message for the agent — starting a loop when none runs,
    /// queued when one does — answered with its fate whenever that
    /// is known, which may be long after the ask; nothing times it
    /// out. What the agent says arrives on the scope's main stream.
    pub async fn enqueue(&self, key: String, content: Vec<ContentBlock>) -> Result<enqueue::response::Frame, UnaryError<Enqueue>> {
        self.enqueue_sent(key, content).await?.await
    }

    /// The same message, handed back once its channel is out and
    /// before its fate is known: the fate is the future, which owns
    /// what it reads and borrows nothing. For a caller whose next ask
    /// must follow this one on the wire.
    pub async fn enqueue_sent(&self, key: String, content: Vec<ContentBlock>) -> Result<BoxFuture<'static, Result<enqueue::response::Frame, UnaryError<Enqueue>>>, UnaryError<Enqueue>> {
        let payload = payload(&channel_request::Frame::Enqueue(enqueue::request::Request { key, content }))
            .map_err(UnaryError::Request)?;
        let channel = self.0.sent(&payload).await.map_err(UnaryError::Send)?;
        Ok(unary_answer::<Enqueue>(channel).boxed())
    }

    /// One more file or directory served into the container at
    /// `container_path`, under `id`, while it runs: the provider's one
    /// answer — made, or why not — once the proxy has answered, and
    /// before any ask of the id. The mount lives until the run ends.
    pub async fn fuse_mount(&self, container_path: Vec<String>, id: String, kind: Kind) -> Result<fuse_mount::response::Frame, UnaryError<FuseMount>> {
        let payload = payload(&channel_request::Frame::FuseMount(fuse_mount::request::Request { container_path, id, kind }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<FuseMount>(&payload).await
    }

    /// Feed the program its secrets' `values`, every name it
    /// declared, at an activation: resolves once the program holds
    /// them, relayed through the provider to the proxy, which
    /// delivers nothing to the program until it does. The first feed
    /// is not this: it answers the registration, before the id. A
    /// refusal is [`Refused`](UnaryError::Refused), the program's
    /// own words through the proxy, and is the activation's error.
    pub async fn secrets(&self, values: Values) -> Result<(), UnaryError<Secrets>> {
        let payload = payload(&channel_request::Frame::Secrets(values)).map_err(UnaryError::Request)?;
        self.0.unary::<Secrets>(&payload).await
    }
}
