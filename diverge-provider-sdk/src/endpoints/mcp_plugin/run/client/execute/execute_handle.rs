//! A plugin that is running, and the end of its run.

use std::fmt;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError;

use super::super::channel_request;
use super::super::super::server::response;
use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};
use crate::decode::Decode;
use crate::frame;
use crate::shared::error::Error;

/// A plugin container, for as long as it exists.
///
/// What [`execute`](super::execute) gives back. The request has gone
/// out, the provider is pulling the image, and the task answering the
/// channels it opens to do that is already running.
///
/// # Stopping is something you say, not something you stop doing
///
/// The scope IS the container's life, and this owns the scope.
/// [`stop`](Self::stop) ends it, and dropping this does not — a
/// reversal, since this used to carry a [`Drop`] that sent the frame. A
/// destructor could not await, could not report a failure, and could
/// not be skipped when a caller wanted the plugin to outlive the value.
///
/// So a caller that drops this without stopping leaves the container
/// running until the connection goes.
///
/// # Two ways to ask how it is going
///
/// [`wait`](Self::wait) blocks until the run is over and says nothing
/// else. [`error`](Self::error) says what ended it, and does not
/// block.
///
/// They are split rather than one method returning a [`Result`] because
/// the questions are asked at different times. A caller that has work
/// to do checks [`error`](Self::error) between pieces of it; a caller
/// with nothing left to do awaits [`wait`](Self::wait). Folding the
/// outcome into `wait`'s return would have left `error` with nothing to
/// report and a second copy of it to keep in step.
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
pub struct ExecuteHandle {
    /// The scope this run opened.
    ///
    /// Private, and there is nothing here that gives it out. Which
    /// means [`stop`](Self::stop) is the only thing that reads it.
    scope: u32,
    /// The scope's responses, until there are no more.
    ///
    /// [`None`] once the run has ended, which is half the terminal
    /// state; [`error`](Self::error) is the other half.
    response_receiver: Option<UnboundedReceiver<Bytes>>,
    /// Why the run ended, if it ended badly.
    ///
    /// Meaningless until `response_receiver` is [`None`], and settled
    /// once and for all when it becomes so. Both
    /// [`wait`](Self::wait) and [`error`](Self::error) can be what sets
    /// it, and they set it through the same path — so it cannot matter
    /// which of them happened to notice.
    error: Option<RunError>,
    /// The write half, for the stop.
    handle: Handle,
}

impl ExecuteHandle {
    /// Take the scope, and the task that serves it.
    ///
    /// Not public. A plugin exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(
        scope: u32,
        response_receiver: UnboundedReceiver<Bytes>,
        handle: Handle,
    ) -> Self {
        ExecuteHandle {
            scope,
            response_receiver: Some(response_receiver),
            error: None,
            handle,
        }
    }

    /// Stop the plugin.
    ///
    /// Returns when the frame has been written, and that is all it
    /// waits for. What ANSWERS a stop is the scope finishing, which is
    /// what [`wait`](Self::wait) is for — so a stop and then a wait is
    /// how a caller sees the container actually gone, and a stop alone
    /// is how it stops caring.
    ///
    /// That pairing is the reason this exists as a method. It could not
    /// be written when stopping was a destructor: dropping sent the
    /// frame and gave up the receiver in the same move, so "stop it and
    /// see it through" was not expressible.
    ///
    /// # What it adds over just going away
    ///
    /// Dropping the connection stops the plugin too, since the scope is
    /// the container's life. The difference is that a provider cannot
    /// tell a deliberate exit from a network that stopped answering,
    /// and has to wait to find out. This is unambiguous and immediate:
    /// a caller that says so is not gone, it is finished.
    ///
    /// # What it does to exchanges in flight
    ///
    /// Ends them, unanswered. A caller with MCP channels still open
    /// when it sends this will see them finish without heads, because
    /// the container they were aimed at is gone. Waiting for them first
    /// is the caller's to do, and nothing here does it on the caller's
    /// behalf.
    ///
    /// # Saying it twice is harmless
    ///
    /// The second one opens another channel and says the same thing,
    /// and a provider that has already finished the scope has no scope
    /// to route it to.
    ///
    /// # The answer is discarded
    ///
    /// Nothing answers a stop; what answers it is the scope's own
    /// finish. So the channel this opens is abandoned as soon as the
    /// frame is out.
    pub async fn stop(&self) -> Result<(), StopError> {
        let mut payload = Vec::new();
        channel_request::Frame::Stop
            .encode(&mut Writer::new(&mut payload))
            .map_err(StopError::Request)?;
        self.handle
            .send_channel_request(self.scope, &payload)
            .await
            .map(|_| ())
            .map_err(StopError::Send)
    }

    /// Wait for the run to be over.
    ///
    /// It does NOT resolve when the plugin comes up. There is no frame
    /// for that and deliberately none — see
    /// [`response::Frame`](super::super::super::server::response::Frame). A
    /// plugin that is working is a scope that says nothing, so this
    /// waits for as long as the plugin runs.
    ///
    /// What ended it is [`error`](Self::error)'s to say, and after this
    /// returns that answer is final.
    ///
    /// # There is no timeout
    ///
    /// Here or anywhere else in this protocol. A plugin that is idle is
    /// a plugin that is running, and a quiet scope is not a finished
    /// one.
    ///
    /// # It waits for an ending, whoever caused it
    ///
    /// A plugin that exits on its own, one that never came up, or one
    /// this caller stopped — [`stop`](Self::stop) and then this is how
    /// a caller sees a stop through, which is the pairing a destructor
    /// could not offer.
    ///
    /// # Waiting again
    ///
    /// Returns at once. The run is over and the reason is already
    /// settled, so there is nothing left to wait for.
    pub async fn wait(&mut self) {
        let Some(responses) = self.response_receiver.as_mut() else {
            return;
        };
        match responses.recv().await {
            Some(bytes) => self.settle(&bytes),
            None => self.end(Some(RunError::Closed)),
        }
    }

    /// What ended the run, if anything has.
    ///
    /// Does not block and does not await. It takes whatever has already
    /// arrived — at most one frame, since one frame is all it takes to
    /// end a run — and reports on it.
    ///
    /// # What [`None`] means
    ///
    /// That nothing has ended the run AS FAR AS THIS HAS SEEN. Two
    /// different things wear that answer:
    ///
    /// - the plugin is still going, which is the ordinary case
    /// - it ended the way it was asked to, and there is nothing to
    ///   report
    ///
    /// They are told apart by whether [`wait`](Self::wait) has
    /// returned. Before it has, [`None`] means "not yet"; after it has,
    /// [`None`] is final and means the plugin stopped cleanly.
    ///
    /// Reporting the two separately would have meant a second state to
    /// carry that says only what asking `wait` already says.
    ///
    /// # Asking twice
    ///
    /// The same answer. A run ends once, and this settles it once.
    pub fn error(&mut self) -> Option<&RunError> {
        if let Some(responses) = self.response_receiver.as_mut() {
            match responses.try_recv() {
                Ok(bytes) => self.settle(&bytes),
                // Still running, and nothing to report about that.
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.end(Some(RunError::Closed));
                }
            }
        }
        self.error.as_ref()
    }

    /// Read the one frame that ends a run, and record what it said.
    ///
    /// Shared by both ways of asking, so that a run cannot end twice or
    /// end differently depending on which of them noticed.
    fn settle(&mut self, bytes: &[u8]) {
        let envelope = match frame::server::ServerFrame::decode(bytes) {
            Ok(envelope) => envelope,
            Err(error) => return self.end(Some(RunError::Frame(error))),
        };
        let payload = match envelope {
            frame::server::ServerFrame::Response { payload, .. } => payload,
            // The finish, which is the run ending as it should.
            frame::server::ServerFrame::ResponseFinish { .. } => {
                return self.end(None);
            }
            _ => return self.end(Some(RunError::Misrouted)),
        };
        self.end(Some(match response::Frame::decode(payload) {
            Ok(response::Frame(error)) => RunError::Provider(error),
            Err(error) => RunError::Response(error),
        }));
    }

    /// Mark the run over, for the given reason or for none.
    ///
    /// Dropping the receiver is what makes it terminal, and it is the
    /// same field [`Drop`] reads to decide whether there is anything
    /// left to stop.
    fn end(&mut self, error: Option<RunError>) {
        self.response_receiver = None;
        self.error = error;
    }
}

/// A stop that did not go out.
///
/// Neither way is the provider refusing — nothing answers a stop, so
/// there is nothing for it to refuse with. Both are this end failing to
/// say it.
#[derive(Debug)]
pub enum StopError {
    /// The request would not serialize.
    ///
    /// Which cannot happen — a stop is one tag byte — and is reported
    /// rather than unwrapped because the encode it shares an impl with
    /// can fail.
    Request(serde_json::Error),
    /// The request never went out.
    ///
    /// See [`SendError`] for the three reasons. Here the one that is
    /// about this exchange rather than the whole connection —
    /// [`Scope`](SendError::Scope) — means the run has already ended,
    /// which is what a stop was trying to arrange.
    Send(SendError),
}

impl fmt::Display for StopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopError::Request(error) => {
                write!(f, "stop did not serialize: {error}")
            }
            StopError::Send(error) => {
                write!(f, "the stop never went out: {error}")
            }
        }
    }
}

impl std::error::Error for StopError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StopError::Request(error) => Some(error),
            StopError::Send(error) => Some(error),
        }
    }
}

/// A run that stopped without ending.
///
/// None of these is the plugin finishing. That is
/// [`error`](ExecuteHandle::error) answering [`None`] once
/// [`wait`](ExecuteHandle::wait) has returned, and the difference is
/// the whole reason this type exists: a run that ended told a caller
/// the plugin is gone, and a run that broke told it nothing about
/// whether it is.
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
