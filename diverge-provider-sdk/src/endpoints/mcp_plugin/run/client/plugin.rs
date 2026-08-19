//! A plugin that is running, and the end of its run.

use std::fmt;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use super::super::server::response;
use crate::client::handle::Handle;
use crate::decode::Decode;
use crate::frame;
use crate::shared::error::Error;

/// A plugin container, for as long as it exists.
///
/// What [`execute`](super::execute) gives back. The request has gone
/// out, the provider is pulling the image, and the task answering the
/// channels it opens to do that is already running.
///
/// # Holding it is what keeps the plugin alive
///
/// The scope IS the container's life, and this owns the scope. Dropping
/// it stops the plugin — see the [`Drop`] impl, which is the ordinary
/// way to be done with one rather than a way to abandon one.
///
/// So a caller keeps this for as long as it means to call the plugin,
/// and lets it go when it is finished. There is nothing else to
/// remember.
///
/// # It is not what you call the plugin WITH
///
/// Calling is a channel request, and this crate has no helper for one
/// yet. A caller sends
/// [`channel_request::Frame::Mcp`](super::channel_request::Frame::Mcp)
/// through the [`Handle`] it already has, quoting
/// [`scope`](Self::scope).
///
/// # Nothing here has to be polled
///
/// Unlike every other `execute` in this crate, which hands back a
/// stream that grows a queue if nobody reads it. A plugin's scope is
/// almost always silent, so there is no backlog to accumulate — and
/// the answering that DOES happen runs on its own task, not on
/// whatever a caller does with this.
#[must_use = "dropping a plugin stops it"]
#[derive(Debug)]
pub struct Plugin {
    /// The scope this run opened.
    ///
    /// Public because it is what a caller needs to say anything to the
    /// plugin. Every channel request naming it belongs to this run,
    /// and it stops meaning anything once this is dropped.
    pub scope: u32,
    /// The scope's responses, until there are no more.
    ///
    /// [`None`] once the run has ended, which is the terminal state and
    /// the whole of it — [`ended`](Self::ended) reads it, and
    /// [`Drop`] checks it to decide whether there is anything left to
    /// stop.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
    /// The write half, for the stop this sends when it is dropped.
    handle: Handle,
    /// The stop, encoded once at construction.
    ///
    /// Because a destructor is no place to serialize: it cannot report
    /// a failure and cannot await, so the bytes are made while there is
    /// still somebody to tell.
    stop_request: Bytes,
    /// The task answering the channels the provider opens.
    ///
    /// Held only to end it. Nothing here waits on it or reads what it
    /// returns — it stops on its own when the scope closes, because the
    /// receiver it reads closes with everything else under a finished
    /// scope.
    serving: JoinHandle<()>,
}

impl Plugin {
    /// Take the scope, and the task that serves it.
    ///
    /// Not public. A plugin exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(
        scope: u32,
        response_receiver: UnboundedReceiver<Bytes>,
        handle: Handle,
        stop_request: Bytes,
        serving: JoinHandle<()>,
    ) -> Self {
        Plugin {
            scope,
            response_receiver: Some(response_receiver),
            handle,
            stop_request,
            serving,
        }
    }

    /// Wait for the run to be over, and learn how it ended.
    ///
    /// [`Ok`] means the run ended: the scope finished, which is what a
    /// provider sends after a stop, and after a plugin exits on its
    /// own.
    ///
    /// It does NOT resolve when the plugin comes up. There is no frame
    /// for that and deliberately none — see
    /// [`response::Frame`](super::super::server::response::Frame). A
    /// plugin that is working is a scope that says nothing, so this
    /// waits for as long as the plugin runs.
    ///
    /// # There is no timeout
    ///
    /// Here or anywhere else in this protocol. A plugin that is idle is
    /// a plugin that is running, and a quiet scope is not a finished
    /// one.
    ///
    /// # The way to STOP a plugin is to drop it
    ///
    /// Which is worth saying because it means "stop it and then wait
    /// for it to finish" cannot be written with what is here: dropping
    /// sends the stop and gives up this receiver in the same move.
    ///
    /// So this is for waiting on an ending somebody else causes — a
    /// plugin that exits, or one that never came up. A caller that
    /// wants to stop one and see it through needs a `stop` that leaves
    /// the handle alive, and there is not one yet.
    ///
    /// # Asking twice
    ///
    /// Answers [`Ok`]. The run is over either way, and how it ended
    /// went to whoever asked first — the terminal state is a field, so
    /// there is nothing left to read it out of.
    pub async fn ended(&mut self) -> Result<(), RunError> {
        let Some(responses) = self.response_receiver.as_mut() else {
            return Ok(());
        };
        let Some(bytes) = responses.recv().await else {
            self.response_receiver = None;
            return Err(RunError::Closed);
        };
        let envelope = match frame::server::ServerFrame::decode(&bytes) {
            Ok(envelope) => envelope,
            Err(error) => {
                self.response_receiver = None;
                return Err(RunError::Frame(error));
            }
        };
        self.response_receiver = None;
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            // The finish, which is the run ending as it should.
            frame::server::ServerFrame::ResponseFinish { .. } => return Ok(()),
            _ => return Err(RunError::Misrouted),
        };
        match response::Frame::decode(payload) {
            Ok(response::Frame(error)) => Err(RunError::Provider(error)),
            Err(error) => Err(RunError::Response(error)),
        }
    }

    /// Whether the run is over.
    ///
    /// Free to answer, because the terminal state is a field rather
    /// than something to work out.
    fn is_ended(&self) -> bool {
        self.response_receiver.is_none()
    }
}

/// Stop the plugin, and stop answering for it.
///
/// # It spawns rather than sends
///
/// A destructor cannot await, and writing a frame means locking a
/// connection and waiting on a socket. What it can do is hand the whole
/// thing to a runtime and return, which is all this does.
///
/// [`try_current`](tokio::runtime::Handle::try_current) rather than
/// [`tokio::spawn`], because `spawn` PANICS outside a runtime and a
/// destructor is the worst place in a program to do that. No runtime
/// means no stop, which leaves things exactly as they were before this
/// existed.
///
/// # It says nothing about a run that ended
///
/// A run that has ended has nothing to stop — and worse, its scope
/// number may since have been handed out again, so a late stop could
/// end somebody else's work. The terminal state is already a field, so
/// the check is free.
///
/// That covers every ending this type observed. What it does not cover
/// is a finish sitting unread in the queue when a caller drops, and
/// that window is closed one level down —
/// [`send_channel_request`](Handle::send_channel_request) looks the
/// scope up after taking back what the router has closed, and a scope
/// that is gone sends nothing.
///
/// # The answer is dropped
///
/// Nothing answers a stop; what answers it is the scope's own finish.
/// So the channel this opens is abandoned immediately, and its entry in
/// the router lingers until the scope closes — which is the thing the
/// stop is provoking.
///
/// # The serving task is aborted, unconditionally
///
/// Even though the stop will end it anyway, moments later. It can land
/// mid-answer, leaving a channel with a head and no body and no finish
/// — which is untidy and is also exactly what the far end would see
/// from a caller that had crashed. The plugin is being torn down either
/// way, and there is no reason to serve an image to a container that is
/// about to stop.
impl Drop for Plugin {
    fn drop(&mut self) {
        self.serving.abort();
        if self.is_ended() {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let handle = self.handle.clone();
        let scope = self.scope;
        let stop_request = self.stop_request.clone();
        runtime.spawn(async move {
            let _ = handle.send_channel_request(scope, &stop_request).await;
        });
    }
}

/// A run that stopped without ending.
///
/// None of these is the plugin finishing. That is [`Ok`] from
/// [`ended`](Plugin::ended), and the difference is the whole reason
/// this type exists: a run that ended told a caller the plugin is gone,
/// and a run that broke told it nothing about whether it is.
///
/// It is flat, and beside [`ExecuteError`](super::ExecuteError) rather
/// than inside it: one is a plugin that never started, this is a plugin
/// whose run stopped without ending.
#[derive(Debug)]
pub enum RunError {
    /// The connection ended mid-run.
    ///
    /// The scope closed without a finish, so whether the plugin is
    /// still running is unknown — not settled either way. A provider
    /// that lost its caller will tear the container down on its own,
    /// but nothing here saw that happen.
    Closed,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them and discards what will not
    /// parse. It is here because
    /// [`Scope`](crate::client::scope::Scope) is public and its
    /// receiver could be fed by something else.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a scope's stream.
    ///
    /// Neither a response nor the finish. Unreachable for the same
    /// reason as [`Frame`](Self::Frame): a router matches on the
    /// frame's type and gives each arm exactly one destination, so a
    /// scope's receiver only ever sees types `2` and `3`.
    ///
    /// Reported rather than treated as the end, because that would be
    /// the one confusion this protocol works hardest to prevent: a run
    /// that BROKE arriving as a run that ENDED.
    Misrouted,
    /// The response frame did not parse.
    ///
    /// Which includes a tag this build does not know — a response kind
    /// added after it was compiled arrives here rather than as
    /// silence.
    Response(response::FrameError),
    /// The plugin did not come up, and the provider said why.
    ///
    /// The image would not pull, the container would not start,
    /// whatever the provider knows. It is the only thing a run's scope
    /// ever says, so this is the only one of these variants that is a
    /// message rather than a mishap.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Closed => {
                f.write_str("connection ended in the middle of a plugin run")
            }
            RunError::Frame(error) => {
                write!(f, "plugin run frame did not decode: {error}")
            }
            RunError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a plugin run",
            ),
            RunError::Response(error) => {
                write!(f, "plugin run frame did not parse: {error}")
            }
            RunError::Provider(_) => f.write_str("the plugin did not come up"),
        }
    }
}

impl std::error::Error for RunError {
    /// [`Provider`](RunError::Provider) has no source, because what it
    /// carries is not a Rust error and deliberately does not implement
    /// one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunError::Frame(error) => Some(error),
            RunError::Response(error) => Some(error),
            RunError::Closed | RunError::Misrouted | RunError::Provider(_) => {
                None
            }
        }
    }
}
