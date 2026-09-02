//! What a server's request frame carries.

use std::error;
use std::fmt;

use super::{fetch_continuation, fetch_directory, fetch_file, fetch_resource};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// The payload of a [`ServerFrame::ChannelRequest`](crate::frame::server::ServerFrame::ChannelRequest).
///
/// One ask toward the client — an MCP exchange toward its proxy, or
/// a fetch ([`FetchFile`](Self::FetchFile) /
/// [`FetchDirectory`](Self::FetchDirectory) /
/// [`FetchResource`](Self::FetchResource) /
/// [`FetchContinuation`](Self::FetchContinuation)) of content the
/// provider is missing. Complete in this frame; the answer comes
/// back as client response frames.
///
/// A payload leads with one byte and the rest is the request.
///
/// What they share is that each is something the server cannot reach
/// itself: the agent runs beside the provider, and the MCP servers —
/// and the store the request's mounts and resources live in — live
/// with the client. So the provider opens a channel, and the client
/// splices the far end into the real thing.
///
/// # MCP is carried as exchanges, not as a socket
///
/// Because MCP over Streamable HTTP is not a connection. It is a series
/// of discrete exchanges over a session identified by a HEADER rather
/// than by anything at the transport layer.
///
/// So there is nothing to tunnel that would not be tunneling a socket
/// for the sake of it. What travels is what was asked: a tool listed, a
/// tool called, a resource read, or the stream a server pushes into.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own JSON.
///
/// # There was no tag, and the reason it has one now
///
/// There was one thing to ask for, and an enum of one variant is a
/// discriminant with nothing to discriminate — so the frame was a
/// struct and the byte that would have said which was not spent.
///
/// That was right, and it stopped being right the moment there were
/// five. They are what a tunneled MCP request used to stand in for: an
/// agent listing tools, listing resources, calling one, reading one,
/// and hearing what the server says unprompted. Naming them is what
/// lets a relay hand over `rmcp`'s own types instead of an HTTP
/// envelope nobody reads.
///
/// The five are prefixed `Mcp`, because a channel a container opens is
/// not necessarily MCP's — a plugin's are a database and a command —
/// and a variant called `CallTool` would only read as MCP's to someone
/// who already knew. The four fetches carry no prefix for the same
/// reason from the other side: they are not MCP exchanges, and names
/// that suggested one would be the same confusion in reverse.
///
/// # The tag is the GET and the POST
///
/// Which is what makes the MCP five the right five rather than an
/// accident. An MCP server has ONE url; a client POSTs a JSON-RPC
/// message to it for the four, and opens a stream with a bare `GET`
/// on the same url for the fifth. The verb is the whole of the
/// distinction there, and the tag byte is the whole of it here. The
/// fetches split by KIND for the same economy: a file, a directory,
/// a resource and the continuation answer with their own frame
/// shapes, and the tag saying which up front is what spares every
/// frame after it a discriminator.
///
/// The frame's own `type` could have carried the discrimination — it
/// is right there in the header — and deliberately does not. A frame
/// already carries one payload's worth of protocol; splitting it
/// across the envelope and the payload would mean two vocabularies to
/// version and two places to keep in step.
///
/// # Header-free, and that is the point
///
/// A tunneled request carries a method, a path and headers that an
/// agent never chose: a provider fabricates them and a terminator
/// discards them. These four carry what was actually asked and nothing
/// else, because both ends already have `rmcp` and the JSON inside
/// that envelope was always `rmcp`'s types.
///
/// # Not the database
///
/// A [`plugin`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres)
/// gets that channel, because a plugin is what needs a database. An
/// agent talks to its tools; a tool is what keeps something. So the
/// tunnel ends where the tool runs, and this loop never sees a
/// connection it has no query to send down.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// What tools are there. Tag `0`.
    ///
    /// [`None`] asks for the first page, which is also what a caller
    /// with nothing to say sends —
    /// [`rmcp`](rmcp::model::ListToolsRequest) makes the params
    /// optional and this keeps that, because params absent and a cursor
    /// absent are different things to a server that reads them.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `1`.
    ///
    /// The same shape as [`McpListTools`](Self::McpListTools), for the same
    /// reason: it is the same MCP request against a different noun.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `2`.
    ///
    /// The name and the arguments, as
    /// [`rmcp`](rmcp::model::CallToolRequestParams) defines them. What
    /// an argument means belongs to the tool, and nothing between here
    /// and it looks.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `3`.
    ///
    /// By URI, as [`rmcp`](rmcp::model::ReadResourceRequestParams)
    /// defines it. The URI is the server's to interpret; a relay that
    /// resolved one would be deciding what a resource is.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `4`.
    ///
    /// Tools changed, resources changed, a resource updated, a log
    /// line. What comes back is one frame per notification for as long
    /// as the channel lives, where the four above answer once — see
    /// [`mcp_notifications`](crate::endpoints::agentic_loop::run::client::channel_response::mcp_notifications).
    ///
    /// # It carries nothing, and MCP is why
    ///
    /// The four above are JSON-RPC methods, and a request that names
    /// one carries its params. This is not a method. In Streamable
    /// HTTP a client opens the notification stream with a bare `GET` on
    /// the same URL it POSTs everything else to — no method name, no
    /// body, nothing to say. There is no `notifications/subscribe` to
    /// mirror.
    ///
    /// So the empty payload is not an economy. It is the request, whole.
    ///
    /// # What it replaces, and what it does not
    ///
    /// It was the last thing a tunneled HTTP exchange could do that
    /// nothing else could, which is what kept one on this endpoint
    /// until there was this.
    ///
    /// Two pieces of that exchange do not come with it. An
    /// `Mcp-Session-Id` said which session a stream belonged to, and
    /// the channel IS the session — opened by one container inside one
    /// scope, belonging to nothing else. A `Last-Event-ID` resumed a
    /// stream whose connection dropped while its session survived, and
    /// here a channel can only die with the scope and a scope only with
    /// the connection, so there is nothing left to resume onto.
    McpNotifications(mcp::notifications::request::Request),
    /// A mounted FILE the provider is missing, by identity. Tag `5`.
    ///
    /// A file the request's `file_mounts` named and the provider does
    /// not hold. The identity alone, as [`fetch_file`] defines it —
    /// not the mount path, because the identity is the content and
    /// the path is only the caller's placement of it.
    ///
    /// Not MCP, which is why it carries no `Mcp` prefix: what answers
    /// it is not a server the client proxies but the client's own
    /// store. The answer is the bytes, verbatim and chunked by
    /// adjacency — see
    /// [`fetch_file`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_file)
    /// — and the empty finish is the client saying it does not hold
    /// the identity.
    FetchFile(fetch_file::Request),
    /// A mounted DIRECTORY the provider is missing, by identity.
    /// Tag `6`.
    ///
    /// The same ask for a `directory_mounts` entry, answered with the
    /// directory's files — one frame per file, adjacent frames per
    /// file where a file is large — see
    /// [`fetch_directory`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_directory).
    FetchDirectory(fetch_directory::Request),
    /// A RESOURCE the provider is missing, by identity. Tag `7`.
    ///
    /// Arbitrary bytes an agent parameter named — a provider
    /// structure's `*_resource` field — under the FILE identity
    /// grammar, as [`fetch_resource`] defines it. The same ask as
    /// [`FetchFile`](Self::FetchFile) for content that is state
    /// rather than a mount: the answer is the bytes, verbatim and
    /// chunked by adjacency — see
    /// [`fetch_resource`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_resource)
    /// — and the empty finish is the client saying it does not hold
    /// the identity.
    FetchResource(fetch_resource::Request),
    /// The continuation the run resumes from. Tag `8`.
    ///
    /// The fetch with nothing to name — a run resumes from the one
    /// continuation its caller holds — so the request carries
    /// nothing, as [`fetch_continuation`] says. The answer is the
    /// bytes, verbatim and chunked by adjacency — see
    /// [`fetch_continuation`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_continuation)
    /// — and the empty finish is a FRESH START, not a refusal.
    FetchContinuation(fetch_continuation::Request),
}

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 0;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 1;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 2;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 3;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 4;

/// Tag for [`Frame::FetchFile`].
const FETCH_FILE: u8 = 5;

/// Tag for [`Frame::FetchDirectory`].
const FETCH_DIRECTORY: u8 = 6;

/// Tag for [`Frame::FetchResource`].
const FETCH_RESOURCE: u8 = 7;

/// Tag for [`Frame::FetchContinuation`].
const FETCH_CONTINUATION: u8 = 8;

/// A tag, then that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. Every variant is serialized, and the
    /// tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out)
            }
            Frame::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out)
            }
            Frame::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out)
            }
            Frame::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out)
            }
            Frame::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::FetchFile(request) => {
                out.extend_from_slice(&[FETCH_FILE]);
                request.encode(out)
            }
            Frame::FetchDirectory(request) => {
                out.extend_from_slice(&[FETCH_DIRECTORY]);
                request.encode(out)
            }
            Frame::FetchResource(request) => {
                out.extend_from_slice(&[FETCH_RESOURCE]);
                request.encode(out)
            }
            Frame::FetchContinuation(request) => {
                out.extend_from_slice(&[FETCH_CONTINUATION]);
                // `Infallible`, as the notification stream's is.
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP_LIST_TOOLS => mcp::list_tools::request::Request::decode(rest)
                .map(Frame::McpListTools)
                .map_err(FrameError::Body),
            MCP_LIST_RESOURCES => {
                mcp::list_resources::request::Request::decode(rest)
                    .map(Frame::McpListResources)
                    .map_err(FrameError::Body)
            }
            MCP_CALL_TOOL => mcp::call_tool::request::Request::decode(rest)
                .map(Frame::McpCallTool)
                .map_err(FrameError::Body),
            MCP_READ_RESOURCE => {
                mcp::read_resource::request::Request::decode(rest)
                    .map(Frame::McpReadResource)
                    .map_err(FrameError::Body)
            }
            MCP_NOTIFICATIONS => Ok(Frame::McpNotifications(
                mcp::notifications::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            FETCH_FILE => fetch_file::Request::decode(rest)
                .map(Frame::FetchFile)
                .map_err(FrameError::Body),
            FETCH_DIRECTORY => fetch_directory::Request::decode(rest)
                .map(Frame::FetchDirectory)
                .map_err(FrameError::Body),
            FETCH_RESOURCE => fetch_resource::Request::decode(rest)
                .map(Frame::FetchResource)
                .map_err(FrameError::Body),
            FETCH_CONTINUATION => Ok(Frame::FetchContinuation(
                fetch_continuation::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    ///
    /// What a provider newer than its caller produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown channel request tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "channel request did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
