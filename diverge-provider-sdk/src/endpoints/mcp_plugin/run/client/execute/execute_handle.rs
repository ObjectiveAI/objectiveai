//! A plugin that is running, and the end of its run.

use std::fmt;

use bytes::Bytes;
use futures_util::{Stream, stream};
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError;

use super::super::channel_request;
use super::super::super::server::channel_response::{
    mcp_call_tool, mcp_list_resources, mcp_list_tools, mcp_notifications,
    mcp_read_resource,
};
use crate::shared::mcp;
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
    /// when it sends this will see them finish without answers, because
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

    /// Ask the plugin what tools it has.
    ///
    /// [`None`] asks for the first page. A plugin with more to give
    /// says so with a cursor, and the next page is another call.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, McpError> {
        let mut payload = Vec::new();
        channel_request::Frame::McpListTools(mcp::list_tools::request::Request(
            params,
        ))
        .encode(&mut Writer::new(&mut payload))
        .map_err(McpError::Request)?;

        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(McpError::Send)?;

        let bytes = channel
            .response_receiver
            .recv()
            .await
            .ok_or(McpError::Unanswered)?;
        let payload = answer(&bytes)?.ok_or(McpError::Unanswered)?;

        match mcp_list_tools::Frame::decode(payload).map_err(McpError::Answer)? {
            mcp_list_tools::Frame::Result(result) => Ok(result),
            mcp_list_tools::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Ask the plugin what resources it has.
    ///
    /// The same shape [`list_tools`](Self::list_tools) has, for the
    /// same reason: it is the same MCP request against a different
    /// noun.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    pub async fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, McpError> {
        let mut payload = Vec::new();
        channel_request::Frame::McpListResources(mcp::list_resources::request::Request(
            params,
        ))
        .encode(&mut Writer::new(&mut payload))
        .map_err(McpError::Request)?;

        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(McpError::Send)?;

        let bytes = channel
            .response_receiver
            .recv()
            .await
            .ok_or(McpError::Unanswered)?;
        let payload = answer(&bytes)?.ok_or(McpError::Unanswered)?;

        match mcp_list_resources::Frame::decode(payload).map_err(McpError::Answer)? {
            mcp_list_resources::Frame::Result(result) => Ok(result),
            mcp_list_resources::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Run one of the plugin's tools.
    ///
    /// # A tool that fails is still [`Ok`]
    ///
    /// [`CallToolResult`] carries its own `is_error`, which is a tool
    /// saying its work did not succeed. An
    /// [`McpError::Mcp`] is the plugin refusing to run it at all: no
    /// such tool, arguments that do not match its schema, a server that
    /// broke. The distinction is MCP's and this keeps it.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    pub async fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        let mut payload = Vec::new();
        channel_request::Frame::McpCallTool(mcp::call_tool::request::Request(
            params,
        ))
        .encode(&mut Writer::new(&mut payload))
        .map_err(McpError::Request)?;

        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(McpError::Send)?;

        let bytes = channel
            .response_receiver
            .recv()
            .await
            .ok_or(McpError::Unanswered)?;
        let payload = answer(&bytes)?.ok_or(McpError::Unanswered)?;

        match mcp_call_tool::Frame::decode(payload).map_err(McpError::Answer)? {
            mcp_call_tool::Frame::Result(result) => Ok(result),
            mcp_call_tool::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Read one of the plugin's resources.
    ///
    /// By URI, which is the plugin's to interpret. Nothing between
    /// here and it resolves one.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    pub async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, McpError> {
        let mut payload = Vec::new();
        channel_request::Frame::McpReadResource(mcp::read_resource::request::Request(
            params,
        ))
        .encode(&mut Writer::new(&mut payload))
        .map_err(McpError::Request)?;

        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(McpError::Send)?;

        let bytes = channel
            .response_receiver
            .recv()
            .await
            .ok_or(McpError::Unanswered)?;
        let payload = answer(&bytes)?.ok_or(McpError::Unanswered)?;

        match mcp_read_resource::Frame::decode(payload).map_err(McpError::Answer)? {
            mcp_read_resource::Frame::Result(result) => Ok(result),
            mcp_read_resource::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Hear what the plugin says on its own account.
    ///
    /// Tools changed, resources changed, a resource updated, a log
    /// line. See [`ServerNotification`] for the whole of what one can
    /// be.
    ///
    /// # It is the one that does not answer
    ///
    /// The other four ask and are answered once. This opens a channel
    /// and reads it for as long as the plugin has anything to push, so
    /// what comes back is a stream rather than a value.
    ///
    /// It ends when the plugin has no more to say, which arrives as the
    /// channel's finish. Dropping the stream drops the channel, and
    /// that is what tells the provider nobody is listening — there is
    /// nothing to unsubscribe with, and nothing needs one.
    ///
    /// # It takes nothing
    ///
    /// Because in MCP there is nothing to ask: a client opens that
    /// stream with a bare `GET` and no body, and there is no
    /// `notifications/subscribe` to mirror.
    ///
    /// # An [`Err`] item is the last one
    ///
    /// Whether it is the plugin saying it will push no more, or this
    /// end failing to read what it pushed. Nothing follows either.
    pub async fn notifications(
        &self,
    ) -> Result<
        impl Stream<Item = Result<ServerNotification, McpError>> + Unpin,
        McpError,
    > {{
        let mut payload = Vec::new();
        channel_request::Frame::McpNotifications(
            mcp::notifications::request::Request,
        )
        .encode(&mut Writer::new(&mut payload))
        .map_err(McpError::Request)?;

        let channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(McpError::Send)?;

        Ok(Box::pin(stream::unfold(Some(channel), |state| async move {{
            let mut channel = state?;
            let bytes = channel.response_receiver.recv().await?;
            let item = notification(&bytes)?;
            // An error is the last thing on this channel, so the state
            // that would read another is dropped with it.
            let next = item.as_ref().err().is_none().then_some(channel);
            Some((item, next))
        }})))
    }}

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
    /// The payload is the provider's error as JSON and nothing else,
    /// so this is that JSON being wrong — there is no tag to be
    /// unknown and no envelope to be malformed.
    Response(serde_json::Error),
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

/// The payload of one channel response, or nothing if it was a finish.
///
/// Whole client frames arrive on a channel's receiver: a session
/// forwards the finish and only then closes the channel, so a reader
/// sees it as an item rather than as the stream ending. Which is what
/// lets a finish mean "that was all" rather than "the connection went".
fn answer(bytes: &[u8]) -> Result<Option<&[u8]>, McpError> {
    match frame::client::ClientFrame::decode(bytes)
        .map_err(McpError::Frame)?
    {
        frame::client::ClientFrame::ChannelResponse { payload, .. } => {
            Ok(Some(payload))
        }
        _ => Ok(None),
    }
}

/// One notification off the stream, or nothing if the channel finished.
///
/// [`None`] ends the stream cleanly, which is what a finish is. An
/// [`Err`] item is a failure that ends it too, and says why.
fn notification(
    bytes: &[u8],
) -> Option<Result<ServerNotification, McpError>> {
    let payload = match answer(bytes) {
        Ok(Some(payload)) => payload,
        // The channel finished, and that is the ordinary end.
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some(
        match mcp_notifications::Frame::decode(payload) {
            Ok(mcp_notifications::Frame::Notification(notification)) => {
                Ok(notification)
            }
            Ok(mcp_notifications::Frame::Error(error)) => {
                Err(McpError::Mcp(error))
            }
            Err(error) => Err(McpError::Answer(error)),
        },
    )
}

/// One MCP exchange that did not happen.
///
/// # It is not what a plugin said
///
/// [`Mcp`](Self::Mcp) is, and it is the only variant that is: the
/// plugin's own MCP server refusing, in its own vocabulary, with a
/// JSON-RPC code that means something. Everything else here is this end
/// failing to ask or failing to read the answer.
///
/// The two are worth telling apart because only one of them says
/// anything about the plugin.
#[derive(Debug)]
pub enum McpError {
    /// The ask would not serialize.
    ///
    /// Which means the params would not, since nothing else in the
    /// frame can fail.
    Request(serde_json::Error),
    /// The ask never went out. See
    /// [`SendError`](crate::client::handle::SendError).
    Send(SendError),
    /// The scope ended before the answer came.
    ///
    /// A provider that finished the scope, or a connection that went.
    /// Also what a channel finishing with nothing on it looks like,
    /// which is what a provider says when it cannot serve the exchange
    /// at all.
    Unanswered,
    /// The answer was not a frame this crate could read.
    Frame(crate::frame::FrameError),
    /// The frame was read and its payload was not an answer.
    Answer(mcp::FrameError),
    /// The plugin's MCP server said no.
    ///
    /// Relayed whole, because the JSON-RPC code is content: `-32601` is
    /// "no such tool" and `-32602` is "the arguments were wrong", and a
    /// caller told only that something failed can act on neither.
    Mcp(ErrorData),
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpError::Request(error) => {
                write!(f, "mcp request did not serialize: {error}")
            }
            McpError::Send(error) => {
                write!(f, "mcp request was not sent: {error}")
            }
            McpError::Unanswered => {
                f.write_str("the scope ended before the plugin answered")
            }
            McpError::Frame(error) => {
                write!(f, "the plugin's answer was not a frame: {error}")
            }
            McpError::Answer(error) => {
                write!(f, "the plugin's answer did not parse: {error}")
            }
            McpError::Mcp(error) => {
                write!(f, "the plugin refused: {error}")
            }
        }
    }
}

impl std::error::Error for McpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            McpError::Request(error) => Some(error),
            McpError::Send(error) => Some(error),
            McpError::Frame(error) => Some(error),
            McpError::Answer(error) => Some(error),
            McpError::Unanswered | McpError::Mcp(_) => None,
        }
    }
}
