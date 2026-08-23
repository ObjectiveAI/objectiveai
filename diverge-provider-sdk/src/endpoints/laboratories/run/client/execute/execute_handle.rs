//! Running a laboratory, and what a runner can say to it.

use std::fmt;
use std::pin::Pin;

use bytes::Bytes;
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedSender;

use super::super::super::server::channel_response;
use super::super::channel_request;
use super::mcp_notification_stream::McpNotificationStream;
use super::read_stream::ReadStream;
use crate::client::handle::{Handle, SendError};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;
use crate::frame;
use crate::shared::container::{read, transfer, write_path};
use crate::shared::error::Error;

/// A laboratory of one's own.
///
/// Half of what [`execute`](super::execute) gives back. The other half
/// is an [`ExecuteStream`](super::ExecuteStream) of what the container
/// is doing, and the two do opposite jobs: that is everything the
/// provider says without being asked, and this is how a runner asks for
/// anything.
///
/// # Stopping is something you say, not something you stop doing
///
/// [`stop`](Self::stop) ends the laboratory, and dropping this does
/// not. So a runner that drops this without stopping leaves the
/// container running until the connection goes.
///
/// And stopping takes everything with it, which is the difference
/// between this and a
/// [`connector leaving`](crate::endpoints::laboratories::connect::client::execute::ExecuteHandle::disconnect).
/// A runner that goes takes nothing: the container runs, the others
/// stay. A runner that stops ends the container, and every runner's
/// scope with it — a connection cannot outlive the thing it joined.
///
/// # Every ask is here
///
/// Five against the container's MCP server —
/// [`list_tools`](Self::list_tools),
/// [`list_resources`](Self::list_resources),
/// [`call_tool`](Self::call_tool),
/// [`read_resource`](Self::read_resource) and
/// [`notifications`](Self::notifications) — and three against the
/// container itself: [`read`](Self::read), [`write`](Self::write) and
/// [`transfer`](Self::transfer). The ninth thing a runner can do is
/// leave, and that is [`Drop`].
///
/// MCP used to be one of them, when it was a tunnel and the ask was a
/// request somebody else had built. It is five now because the wire
/// carries the exchanges themselves, and five exchanges cannot be one
/// method without inventing a union of them.
///
/// They are three shapes, not one. The four MCP asks and a transfer are
/// one ask and one answer, so they resolve to a result. A read and
/// [`notifications`](Self::notifications) are one ask and a stream, so
/// they hand one back. A write is one ask whose CONTENT travels the
/// other way, on a channel the provider opens, which is why it is the
/// only one that needed anything built.
#[must_use = "dropping the handle leaves the laboratory"]
#[derive(Debug)]
pub struct ExecuteHandle {
    /// What everything a runner says goes out over.
    ///
    /// A [`Handle`] rather than pre-encoded frames.
    /// [`send_channel_request`](Handle::send_channel_request) takes
    /// back what the router has closed and then looks the scope up, so
    /// a laboratory that has already ended sends nothing rather than
    /// reaching into whoever holds that scope number now.
    handle: Handle,
    /// The scope this laboratory is running in, as this end numbered
    /// it.
    ///
    /// Every channel a runner opens carries it, and so does the stop.
    /// The router already sorted the incoming frames by it, so nothing
    /// here reads it for that.
    scope: u32,
    /// Where a write's content is handed to the task that serves it.
    ///
    /// [`write`](Self::write) puts one here BEFORE it asks for the
    /// write, which is what makes the race a non-race: the provider
    /// cannot ask for content until it has the request, and the request
    /// goes out after this.
    write_sender: UnboundedSender<Write>,
}

impl ExecuteHandle {
    /// Take the pieces, from the [`execute`](super::execute) that has
    /// them.
    ///
    /// Not public. A connection exists because a request went out, so
    /// the only thing that can honestly make one of these is the thing
    /// that sent it.
    pub(super) fn new(
        handle: Handle,
        scope: u32,
        write_sender: UnboundedSender<Write>,
    ) -> Self {
        ExecuteHandle {
            handle,
            scope,
            write_sender,
        }
    }

    /// Stop the container.
    ///
    /// Returns when the frame has been written, and that is all it
    /// waits for. What ANSWERS a stop is the scope's own finish, which
    /// arrives on the [`ExecuteStream`](super::ExecuteStream) as
    /// [`None`] — so a runner that wants to see the container actually
    /// gone reads the stream until it ends, and one that only wants to
    /// stop paying for it is finished here.
    ///
    /// # It ends everybody, not only this scope
    ///
    /// Connectors hold scopes on a container that will not exist, so
    /// those scopes finish too. A connection cannot outlive the thing
    /// it joined, and nothing is asked of the connectors first — a
    /// runner that wants them warned warns them itself, out of band,
    /// because this protocol gives it no way to.
    ///
    /// # What it adds over just going away
    ///
    /// The scope IS the container's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a runner
    /// that says so is not gone, it is finished.
    ///
    /// # Exchanges in flight end where they are
    ///
    /// A read still arriving, an MCP event stream, a write still
    /// streaming its content — all of them stop when the scope does,
    /// unfinished. Waiting for them first is a runner's to do, and
    /// nothing here does it on a runner's behalf.
    ///
    /// # Saying it twice is harmless
    ///
    /// The second one opens another channel and says the same thing,
    /// and a provider that has already finished the scope has no scope
    /// to route it to. Nothing here refuses it, because nothing here
    /// knows — the terminal state lives on the stream, which a runner
    /// may have dropped.
    ///
    /// # The answer is discarded
    ///
    /// Nothing answers a stop; what answers it is the scope's own
    /// finish. So the channel this opens is abandoned as soon as the
    /// frame is out, and its entry in the router lingers until the
    /// scope closes — which is the thing the stop is provoking.
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

    /// Ask what tools the container offers.
    ///
    /// [`None`] asks for the first page. A server with more to give
    /// says so with a cursor, and the next page is another call.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    ///
    /// # Several at once are fine
    ///
    /// Each takes its own channel, so exchanges do not queue behind one
    /// another and a slow tool does not hold up a listing. Their frames
    /// interleave on the socket, which is what keeps that true.
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
        let envelope = frame::server::ServerFrame::decode(&bytes)
            .map_err(McpError::Frame)?;
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // A finish with nothing before it, which is what a provider
            // says when it cannot serve the exchange at all.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                return Err(McpError::Unanswered);
            }
            _ => return Err(McpError::Misrouted),
        };

        match channel_response::mcp_list_tools::Frame::decode(payload).map_err(McpError::Answer)? {
            channel_response::mcp_list_tools::Frame::Result(result) => Ok(result),
            channel_response::mcp_list_tools::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Ask what resources the container offers.
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
    ///
    /// # Several at once are fine
    ///
    /// Each takes its own channel, so exchanges do not queue behind one
    /// another and a slow tool does not hold up a listing. Their frames
    /// interleave on the socket, which is what keeps that true.
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
        let envelope = frame::server::ServerFrame::decode(&bytes)
            .map_err(McpError::Frame)?;
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // A finish with nothing before it, which is what a provider
            // says when it cannot serve the exchange at all.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                return Err(McpError::Unanswered);
            }
            _ => return Err(McpError::Misrouted),
        };

        match channel_response::mcp_list_resources::Frame::decode(payload).map_err(McpError::Answer)? {
            channel_response::mcp_list_resources::Frame::Result(result) => Ok(result),
            channel_response::mcp_list_resources::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Run one of the container's tools.
    ///
    /// # A tool that fails is still [`Ok`]
    ///
    /// [`CallToolResult`] carries its own `is_error`, which is a tool
    /// saying its work did not succeed. An [`McpError::Mcp`] is the
    /// server refusing to run it at all: no such tool, arguments that
    /// do not match its schema, a server that broke. The distinction is
    /// MCP's and this keeps it.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    ///
    /// # Several at once are fine
    ///
    /// Each takes its own channel, so exchanges do not queue behind one
    /// another and a slow tool does not hold up a listing. Their frames
    /// interleave on the socket, which is what keeps that true.
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
        let envelope = frame::server::ServerFrame::decode(&bytes)
            .map_err(McpError::Frame)?;
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // A finish with nothing before it, which is what a provider
            // says when it cannot serve the exchange at all.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                return Err(McpError::Unanswered);
            }
            _ => return Err(McpError::Misrouted),
        };

        match channel_response::mcp_call_tool::Frame::decode(payload).map_err(McpError::Answer)? {
            channel_response::mcp_call_tool::Frame::Result(result) => Ok(result),
            channel_response::mcp_call_tool::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Read one of the container's resources.
    ///
    /// By URI, which is the server's to interpret. Nothing between
    /// here and it resolves one.
    ///
    /// # One channel, one answer
    ///
    /// This opens a channel, writes the ask, and reads the one frame
    /// that answers it. The channel finishes after, which is the
    /// provider's to do and not something a caller waits for — so the
    /// channel is dropped here rather than drained.
    ///
    /// # Several at once are fine
    ///
    /// Each takes its own channel, so exchanges do not queue behind one
    /// another and a slow tool does not hold up a listing. Their frames
    /// interleave on the socket, which is what keeps that true.
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
        let envelope = frame::server::ServerFrame::decode(&bytes)
            .map_err(McpError::Frame)?;
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            // A finish with nothing before it, which is what a provider
            // says when it cannot serve the exchange at all.
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                return Err(McpError::Unanswered);
            }
            _ => return Err(McpError::Misrouted),
        };

        match channel_response::mcp_read_resource::Frame::decode(payload).map_err(McpError::Answer)? {
            channel_response::mcp_read_resource::Frame::Result(result) => Ok(result),
            channel_response::mcp_read_resource::Frame::Error(error) => Err(McpError::Mcp(error)),
        }
    }

    /// Hear what the container says on its own account.
    ///
    /// Tools changed, resources changed, a resource updated, a log
    /// line. What comes back is an [`McpNotificationStream`], and the
    /// provider is already putting into it.
    ///
    /// # It is the one ask that is not answered
    ///
    /// The other four are asked once and answered once. This is not an
    /// answer: it is the place a server pushes into, and it produces
    /// for as long as the stream is held.
    ///
    /// Dropping the stream drops the channel, and that is what tells
    /// the provider nobody is listening. There is nothing to
    /// unsubscribe with and nothing needs one.
    ///
    /// # It takes nothing
    ///
    /// Because in MCP there is nothing to ask: a client opens that
    /// stream with a bare `GET` and no body, and there is no
    /// `notifications/subscribe` to mirror.
    ///
    /// # Returning is not hearing
    ///
    /// This resolves once the ask has gone out. Whether the server has
    /// anything to say, and when, is the stream's to report.
    pub async fn notifications(
        &self,
    ) -> Result<McpNotificationStream, McpError> {{
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

        Ok(McpNotificationStream::new(channel.response_receiver))
    }}

    /// Read one file out of the container.
    ///
    /// Returns once the request has gone out, not once the file has
    /// arrived — what comes back is a [`ReadStream`] of the file's
    /// pieces, and the provider is already sending into it.
    ///
    /// # One channel, and only the provider talks on it
    ///
    /// Which is what makes a read the simplest of the three. A
    /// connector says which file; the provider answers with the bytes
    /// and finishes. Nothing has to travel back the other way, so
    /// nothing had to be inverted the way a
    /// [`write`](Self::write) is.
    ///
    /// # It reads one file, and never a directory
    ///
    /// See [`read`](crate::shared::container::read) for why. A tree is
    /// not something this exchange can carry, and asking for one is not
    /// an error it reports — the path names a file or the read fails.
    ///
    /// # Several at once are fine
    ///
    /// Each takes its own channel, so reads do not queue behind one
    /// another and a large file does not hold up a small one. Their
    /// frames interleave on the socket, which is what keeps that true.
    pub async fn read(
        &self,
        request: read::request::Request,
    ) -> Result<ReadStream, ReadError> {
        let mut payload = Vec::new();
        channel_request::Frame::Read(request)
            .encode(&mut Writer::new(&mut payload))
            .map_err(ReadError::Request)?;
        let channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(ReadError::Send)?;
        Ok(ReadStream::new(channel.response_receiver))
    }

    /// Move one file into another container.
    ///
    /// Resolves when the provider says the file is at the destination,
    /// or says it is not.
    ///
    /// # Nothing travels
    ///
    /// Not through this connection, and not through the connector. Both
    /// containers are the provider's, so the bytes go from one to the
    /// other without ever becoming a message — which is the whole point
    /// of having this rather than a
    /// [`read`](Self::read) piped into a [`write`](Self::write).
    ///
    /// Which also means a transfer is the one of the three that is one
    /// ask and one answer. There is no content to stream in either
    /// direction, so there is no stream.
    ///
    /// # The destination is named, not held
    ///
    /// See [`transfer`](crate::shared::container::transfer) for when a
    /// provider can do this at all. A destination it does not host is
    /// not something a runner can work around from here, and it
    /// comes back as an error like any other refusal.
    pub async fn transfer(
        &self,
        request: transfer::request::Request,
    ) -> Result<(), TransferError> {
        let mut payload = Vec::new();
        channel_request::Frame::Transfer(request)
            .encode(&mut Writer::new(&mut payload))
            .map_err(TransferError::Request)?;
        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(TransferError::Send)?;

        let Some(bytes) = channel.response_receiver.recv().await else {
            return Err(TransferError::Closed);
        };
        let envelope = frame::server::ServerFrame::decode(&bytes)
            .map_err(TransferError::Frame)?;
        let payload = match envelope {
            frame::server::ServerFrame::ChannelResponse { payload, .. } => {
                payload
            }
            frame::server::ServerFrame::ChannelResponseFinish { .. } => {
                return Err(TransferError::Unanswered);
            }
            _ => return Err(TransferError::Misrouted),
        };
        match channel_response::transfer::Frame::decode(payload) {
            Ok(channel_response::transfer::Frame::Transferred(_)) => Ok(()),
            Ok(channel_response::transfer::Frame::Error(error)) => {
                Err(TransferError::Provider(error))
            }
            Err(error) => Err(TransferError::Response(error)),
        }
    }

    /// Write one file into the container.
    ///
    /// Resolves when the provider says the file is at the path, or says
    /// it is not. The content is streamed while that is being waited
    /// on.
    ///
    /// # Two channels, and this only opens one of them
    ///
    /// A write names its destination on a channel a runner opens,
    /// and its CONTENT comes back on a channel the PROVIDER opens.
    /// Which looks backwards until you notice that only a responder can
    /// finish a channel: content is a stream, a stream needs an end,
    /// and a runner asking has no frame with which to say it has
    /// stopped.
    ///
    /// So this sends the request and waits. Somewhere else, a task
    /// takes the provider's ask for content and answers it with what
    /// was handed over here — which is why the content is a parameter
    /// rather than something written through the returned value.
    ///
    /// # The stream, and what an [`Err`] in it does
    ///
    /// An [`Err`] ends the write. It goes out as an
    /// [`Error`](super::super::channel_response::write_bytes::Frame::Error) on
    /// the content channel, the channel finishes, and nothing after it
    /// in the stream is read — which is what that frame means: "the
    /// full content was not streamed".
    ///
    /// It does not end THIS call. A provider that has been told the
    /// content stopped still answers the write, and what it answers is
    /// what comes back here.
    ///
    /// [`Send`] and `'static` because the stream outlives this call —
    /// it is read by the task that answers the provider, not by this.
    /// [`Sync`] is not required, here or on any of the
    /// [`client`](crate::client) proxies: one task owns the stream and
    /// polls it through `&mut`, so a shared reference to it never
    /// exists.
    ///
    /// # The id is the caller's, and so is keeping it unique
    ///
    /// [`write_id`](write_path::request::Request::write_id) is what
    /// ties the provider's ask for content back to the content handed
    /// over here, and nothing checks it. Two outstanding writes sharing
    /// one makes them indistinguishable, and this end is the only party
    /// that could have prevented it — the same obligation
    /// [`write_path::request::Request`] states about itself. Reuse
    /// after a write has finished is fine.
    ///
    /// # What a partial write leaves behind
    ///
    /// Nothing at the destination, whether this answers or not. A
    /// provider writes to a temporary and renames it into place, so the
    /// path holds the old file, then nothing, then the new one — never
    /// a prefix of the new one.
    pub async fn write<S>(
        &self,
        request: write_path::request::Request,
        content: S,
    ) -> Result<(), WriteError>
    where
        S: Stream<Item = Result<Bytes, Error>> + Send + 'static,
    {
        let write_id = request.write_id;
        let mut payload = Vec::new();
        channel_request::Frame::Write(request)
            .encode(&mut Writer::new(&mut payload))
            .map_err(WriteError::Request)?;

        // Before the request goes out, so the provider's ask for
        // content cannot arrive before the content it names.
        self.write_sender
            .send(Write {
                write_id,
                content: Box::pin(content),
            })
            .map_err(|_| WriteError::Serving)?;

        let mut channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(WriteError::Send)?;

        loop {
            let Some(bytes) = channel.response_receiver.recv().await else {
                return Err(WriteError::Closed);
            };
            let envelope = frame::server::ServerFrame::decode(&bytes)
                .map_err(WriteError::Frame)?;
            let payload = match envelope {
                frame::server::ServerFrame::ChannelResponse {
                    payload, ..
                } => payload,
                frame::server::ServerFrame::ChannelResponseFinish {
                    ..
                } => return Err(WriteError::Unanswered),
                _ => return Err(WriteError::Misrouted),
            };
            return match channel_response::write_path::Frame::decode(payload)
            {
                Ok(channel_response::write_path::Frame::Written(_)) => Ok(()),
                Ok(channel_response::write_path::Frame::Error(error)) => {
                    Err(WriteError::Provider(error))
                }
                Err(error) => Err(WriteError::Response(error)),
            };
        }
    }
}

/// One write's content, on its way to the task that will answer for it.
///
/// Not public. It exists because the ask for content and the content
/// itself arrive at the connection from opposite directions, and
/// something has to hold one until the other shows up.
pub(super) struct Write {
    /// Which write this is the content for.
    pub(super) write_id: u32,
    /// The content.
    ///
    /// Boxed because it is one of many shapes and has to sit in a queue
    /// beside the others. `Send` and `'static` and NOT `Sync`, because
    /// exactly one task ever holds it.
    pub(super) content:
        Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>,
}

/// Written out rather than derived, because a stream is not
/// [`Debug`](fmt::Debug) and requiring that of a caller's would be a
/// bound charged for a line of output nobody reads.
impl fmt::Debug for Write {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Write")
            .field("write_id", &self.write_id)
            .finish_non_exhaustive()
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
    /// [`Scope`](SendError::Scope) — means the laboratory has already
    /// ended, which is what a stop was trying to arrange.
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

/// One MCP exchange that did not happen.
///
/// # It is not what the server said
///
/// [`Mcp`](Self::Mcp) is, and it is the only variant that is: the
/// container's own MCP server refusing, in its own vocabulary, with a
/// JSON-RPC code that means something. Everything else here is this end
/// failing to ask or failing to read the answer.
///
/// The two are worth telling apart because only one of them says
/// anything about the container.
///
/// It used to be two variants and no answers at all, because the
/// exchange was HTTP and a refusal was a status on a stream that was
/// working. There is no status now, so a refusal has to be sayable.
#[derive(Debug)]
pub enum McpError {
    /// The ask would not serialize.
    ///
    /// Which means the params would not, since nothing else in the
    /// frame can fail.
    Request(serde_json::Error),
    /// The ask never went out.
    ///
    /// See [`SendError`] for the three reasons, only one of which is
    /// about this exchange rather than the whole connection.
    Send(SendError),
    /// The scope ended before the answer came.
    ///
    /// A provider that finished the scope, or a connection that went.
    /// Also what a channel finishing with nothing on it looks like,
    /// which is what a provider says when it cannot serve the exchange
    /// at all.
    Unanswered,
    /// The answer was not a frame this crate could read.
    Frame(frame::FrameError),
    /// A frame arrived that has no business on this channel.
    ///
    /// A router puts only a channel response and its finish here, so
    /// anything else is a provider or a router disagreeing with this
    /// one about what a channel is.
    Misrouted,
    /// The frame was read and its payload was not an answer.
    Answer(mcp::FrameError),
    /// The container's MCP server said no.
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
                f.write_str("the scope ended before the server answered")
            }
            McpError::Frame(error) => {
                write!(f, "the answer was not a frame: {error}")
            }
            McpError::Misrouted => {
                f.write_str("a frame arrived that was not an mcp answer")
            }
            McpError::Answer(error) => {
                write!(f, "the answer did not parse: {error}")
            }
            McpError::Mcp(error) => {
                write!(f, "the server refused: {error}")
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
            McpError::Unanswered
            | McpError::Misrouted
            | McpError::Mcp(_) => None,
        }
    }
}

/// A read that never started.
///
/// Two ways, and neither of them is the provider refusing — a file it
/// cannot read arrives as
/// [`ReadStreamError::Provider`](super::ReadStreamError::Provider), on
/// a channel that opened to carry it.
#[derive(Debug)]
pub enum ReadError {
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The request never went out.
    ///
    /// See [`SendError`] for the three reasons, only one of which is
    /// about this exchange rather than the whole connection.
    Send(SendError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Request(error) => {
                write!(f, "read request did not serialize: {error}")
            }
            ReadError::Send(error) => {
                write!(f, "the read request never went out: {error}")
            }
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Request(error) => Some(error),
            ReadError::Send(error) => Some(error),
        }
    }
}

/// A transfer that did not happen, or could not be asked for.
///
/// Only [`Provider`](Self::Provider) is the provider refusing. The rest
/// are the exchange going wrong around it, and none of them says
/// anything about what is at the destination — a transfer that fails
/// leaves nothing partial there, whether this heard so or not.
#[derive(Debug)]
pub enum TransferError {
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The request never went out.
    ///
    /// See [`SendError`] for the three reasons, only one of which is
    /// about this exchange rather than the whole connection.
    Send(SendError),
    /// The channel closed without an answer.
    ///
    /// The connection went away, or the scope did. Whether the file
    /// moved is unknown — the provider may have finished moving it
    /// after this end stopped being able to hear so.
    Closed,
    /// The provider finished the channel without answering.
    ///
    /// Distinct from [`Closed`](Self::Closed): the connection is fine
    /// and the provider deliberately said nothing, which is not
    /// something this protocol gives it a way to mean.
    Unanswered,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a channel's answer.
    Misrouted,
    /// The answer did not parse.
    Response(channel_response::transfer::FrameError),
    /// The provider says the file is not at the destination.
    ///
    /// Which includes the case it could never have done: a destination
    /// this provider does not host is a refusal like any other, because
    /// a transfer that never leaves the provider cannot reach one that
    /// is somewhere else.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Provider(Error),
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferError::Request(error) => {
                write!(f, "transfer request did not serialize: {error}")
            }
            TransferError::Send(error) => {
                write!(f, "the transfer request never went out: {error}")
            }
            TransferError::Closed => f.write_str(
                "the connection ended before the transfer was answered",
            ),
            TransferError::Unanswered => {
                f.write_str("the transfer channel finished without an answer")
            }
            TransferError::Frame(error) => {
                write!(f, "transfer answer did not decode: {error}")
            }
            TransferError::Misrouted => f.write_str(
                "a frame arrived that does not belong on a transfer",
            ),
            TransferError::Response(error) => {
                write!(f, "transfer answer did not parse: {error}")
            }
            TransferError::Provider(_) => {
                f.write_str("the file was not transferred")
            }
        }
    }
}

impl std::error::Error for TransferError {
    /// [`Provider`](TransferError::Provider) has no source, because
    /// what it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TransferError::Request(error) => Some(error),
            TransferError::Send(error) => Some(error),
            TransferError::Frame(error) => Some(error),
            TransferError::Response(error) => Some(error),
            TransferError::Closed
            | TransferError::Unanswered
            | TransferError::Misrouted
            | TransferError::Provider(_) => None,
        }
    }
}

/// A write that did not land, or could not be asked for.
///
/// Only [`Provider`](Self::Provider) is the provider refusing. The rest
/// are the exchange going wrong around it, and none of them says
/// anything about what is at the path — see
/// [`write_path::response::Frame`] for why that is safe either way.
#[derive(Debug)]
pub enum WriteError {
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The task that answers content requests has stopped.
    ///
    /// Which means the scope has ended, since that is the only thing
    /// that stops it. Nothing was sent, and nothing would have been
    /// answered if it had been.
    Serving,
    /// The request never went out.
    ///
    /// See [`SendError`] for the three reasons, only one of which is
    /// about this exchange rather than the whole connection.
    Send(SendError),
    /// The channel closed without an answer.
    ///
    /// The connection went away, or the scope did. Whether the file
    /// landed is unknown — the provider may have finished writing it
    /// after this end stopped being able to hear so.
    Closed,
    /// The provider finished the channel without answering.
    ///
    /// Distinct from [`Closed`](Self::Closed): the connection is fine
    /// and the provider deliberately said nothing, which is not
    /// something this protocol gives it a way to mean.
    Unanswered,
    /// What came back was not a frame.
    ///
    /// Unreachable through this crate's own
    /// [`Router`](crate::client::router::Router), which decodes the
    /// same bytes before forwarding them.
    Frame(frame::FrameError),
    /// A frame arrived that does not belong on a channel's answer.
    Misrouted,
    /// The answer did not parse.
    Response(channel_response::write_path::FrameError),
    /// The provider says the file is not at the path.
    ///
    /// And nothing partial is either. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Provider(Error),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::Request(error) => {
                write!(f, "write request did not serialize: {error}")
            }
            WriteError::Serving => {
                f.write_str("the connection is no longer serving write_sender")
            }
            WriteError::Send(error) => {
                write!(f, "the write request never went out: {error}")
            }
            WriteError::Closed => {
                f.write_str("the connection ended before the write was answered")
            }
            WriteError::Unanswered => {
                f.write_str("the write channel finished without an answer")
            }
            WriteError::Frame(error) => {
                write!(f, "write answer did not decode: {error}")
            }
            WriteError::Misrouted => {
                f.write_str("a frame arrived that does not belong on a write")
            }
            WriteError::Response(error) => {
                write!(f, "write answer did not parse: {error}")
            }
            WriteError::Provider(_) => {
                f.write_str("the file was not written")
            }
        }
    }
}

impl std::error::Error for WriteError {
    /// [`Provider`](WriteError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::Request(error) => Some(error),
            WriteError::Send(error) => Some(error),
            WriteError::Frame(error) => Some(error),
            WriteError::Response(error) => Some(error),
            WriteError::Serving
            | WriteError::Closed
            | WriteError::Unanswered
            | WriteError::Misrouted
            | WriteError::Provider(_) => None,
        }
    }
}
