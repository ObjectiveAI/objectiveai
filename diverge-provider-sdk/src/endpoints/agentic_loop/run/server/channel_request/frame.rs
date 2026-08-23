//! What a server's request frame carries.

use std::error;
use std::fmt;

use rmcp::model::{
    CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams,
};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// The payload of a [`ServerFrame::ChannelRequest`](crate::frame::server::ServerFrame::ChannelRequest).
///
/// One MCP exchange, toward the client's MCP proxy. Complete in this
/// frame; the answer comes back as client response frames.
///
/// A payload leads with one byte and the rest is the request.
///
/// It is the one thing a server asks its client for, and it is a
/// connection the server cannot make itself: the agent runs beside the
/// provider, and the MCP servers live with the client. So the provider
/// opens a channel, and the client splices the far end into the real
/// thing.
///
/// # MCP is carried as exchanges, not as a socket
///
/// Because MCP over Streamable HTTP is not a connection. It is a
/// series of discrete exchanges over a session identified by a HEADER
/// rather than by anything at the transport layer.
///
/// Terminating the HTTP at each end and carrying the exchange itself
/// keeps HTTP/1.1 framing out of this protocol entirely: no chunked
/// encoding, no keep-alive boundaries, no request parser in the
/// conduit, and a terminator that can rebuild an ordinary request and
/// hand it to an ordinary router. The JSON-RPC inside stays opaque
/// regardless — see
/// [`Request::body`](crate::shared::http::request::Request::body).
///
/// # A struct, and no tag
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
/// six. The five beside [`Mcp`](Self::Mcp) are what a tunneled MCP
/// request was standing in for: an agent listing tools, listing
/// resources, calling one, reading one, and hearing what the server
/// says unprompted. Naming them is what lets a relay hand over
/// `rmcp`'s own types instead of an HTTP envelope nobody reads.
///
/// # The tag is the GET and the POST
///
/// Which is what makes six the right number rather than an accident.
/// An MCP server has ONE url; a client POSTs a JSON-RPC message to it
/// for the four, and opens a stream with a bare `GET` on the same url
/// for the fifth. The verb is the whole of the distinction there, and
/// the tag byte is the whole of it here.
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
pub enum Frame<'a> {
    /// One MCP exchange, tunneled. Tag `0`.
    ///
    /// The request, relayed verbatim. Handed off to
    /// [`Request`](crate::shared::http::request::Request)'s own impl
    /// rather than serialized here — not to save the four lines, but
    /// because an MCP request has ONE wire form, and writing it a
    /// second time in a second place is how two wire forms start.
    ///
    /// # It is being replaced
    ///
    /// By the four beneath it, which say what they are asking for
    /// instead of carrying a request that says it. What keeps this
    /// alive is what those four cannot do yet: an event stream, which
    /// is how a server pushes notifications and answers things it was
    /// asked while it was thinking.
    ///
    /// When that has a variant of its own, this goes.
    Mcp(Request<'a>),
    /// What tools are there. Tag `1`.
    ///
    /// [`None`] asks for the first page, which is also what a caller
    /// with nothing to say sends —
    /// [`rmcp`](rmcp::model::ListToolsRequest) makes the params
    /// optional and this keeps that, because params absent and a cursor
    /// absent are different things to a server that reads them.
    ListTools(Option<PaginatedRequestParams>),
    /// What resources are there. Tag `2`.
    ///
    /// The same shape as [`ListTools`](Self::ListTools), for the same
    /// reason: it is the same MCP request against a different noun.
    ListResources(Option<PaginatedRequestParams>),
    /// Run one tool. Tag `3`.
    ///
    /// The name and the arguments, as
    /// [`rmcp`](rmcp::model::CallToolRequestParams) defines them. What
    /// an argument means belongs to the tool, and nothing between here
    /// and it looks.
    CallTool(CallToolRequestParams),
    /// Read one resource. Tag `4`.
    ///
    /// By URI, as [`rmcp`](rmcp::model::ReadResourceRequestParams)
    /// defines it. The URI is the server's to interpret; a relay that
    /// resolved one would be deciding what a resource is.
    ReadResource(ReadResourceRequestParams),
    /// Everything the server says on its own account. Tag `5`.
    ///
    /// Tools changed, resources changed, a resource updated, a log
    /// line. What comes back is one frame per notification for as long
    /// as the channel lives, where the four above answer once — see
    /// [`notifications`](crate::endpoints::agentic_loop::run::client::channel_response::notifications).
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
    /// It is the last thing [`Mcp`](Self::Mcp) could do that nothing
    /// else could, which is what kept a tunneled HTTP exchange alive.
    ///
    /// Two pieces of that exchange do not come with it. An
    /// `Mcp-Session-Id` said which session a stream belonged to, and
    /// the channel IS the session — opened by one container inside one
    /// scope, belonging to nothing else. A `Last-Event-ID` resumed a
    /// stream whose connection dropped while its session survived, and
    /// here a channel can only die with the scope and a scope only with
    /// the connection, so there is nothing left to resume onto.
    Notifications,
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

/// Tag for [`Frame::ListTools`].
const LIST_TOOLS: u8 = 1;

/// Tag for [`Frame::ListResources`].
const LIST_RESOURCES: u8 = 2;

/// Tag for [`Frame::CallTool`].
const CALL_TOOL: u8 = 3;

/// Tag for [`Frame::ReadResource`].
const READ_RESOURCE: u8 = 4;

/// Tag for [`Frame::Notifications`].
const NOTIFICATIONS: u8 = 5;

/// A tag, then that variant's own JSON.
impl Encode for Frame<'_> {
    /// The ordinary JSON failure. Every variant is serialized, and the
    /// tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Mcp(request) => {
                out.extend_from_slice(&[MCP]);
                request.encode(out)
            }
            Frame::ListTools(params) => {
                out.extend_from_slice(&[LIST_TOOLS]);
                serde_json::to_writer(out, params)
            }
            Frame::ListResources(params) => {
                out.extend_from_slice(&[LIST_RESOURCES]);
                serde_json::to_writer(out, params)
            }
            Frame::CallTool(params) => {
                out.extend_from_slice(&[CALL_TOOL]);
                serde_json::to_writer(out, params)
            }
            Frame::ReadResource(params) => {
                out.extend_from_slice(&[READ_RESOURCE]);
                serde_json::to_writer(out, params)
            }
            Frame::Notifications => {
                out.extend_from_slice(&[NOTIFICATIONS]);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest).map(Frame::Mcp).map_err(FrameError::Body),
            LIST_TOOLS => serde_json::from_slice(rest)
                .map(Frame::ListTools)
                .map_err(FrameError::Body),
            LIST_RESOURCES => serde_json::from_slice(rest)
                .map(Frame::ListResources)
                .map_err(FrameError::Body),
            CALL_TOOL => serde_json::from_slice(rest)
                .map(Frame::CallTool)
                .map_err(FrameError::Body),
            READ_RESOURCE => serde_json::from_slice(rest)
                .map(Frame::ReadResource)
                .map_err(FrameError::Body),
            // Whatever follows the tag is ignored rather than rejected.
            // There is nothing this variant could carry, so a reader
            // that found something has met a writer from a version that
            // gave it one — and the tag already said what was meant.
            NOTIFICATIONS => Ok(Frame::Notifications),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
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
                f.write_str("mcp channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp channel request tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "mcp channel request did not parse: {error}")
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
