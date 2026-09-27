//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

use super::{agents, tools};

/// The payload of a
/// [`Request`](crate::wire::frame::client::ClientFrame::Request).
///
/// One variant per scope, in tag order, plus
/// [`Invalid`](Self::Invalid). This is the only place the tag values
/// meet: each request states its own in its own module, and nothing
/// else can see all of them at once — so this is what the table in
/// [`endpoints`](super) is a table OF.
///
/// # Decoding cannot fail
///
/// There is no error type. A payload this does not understand — an
/// unknown tag, a body that will not parse, no bytes at all — becomes
/// [`Invalid`](Self::Invalid) rather than a failure, so the daemon
/// always has a request to answer and a malformed one is a scope like
/// any other. The alternative, refusing to decode, would leave the
/// daemon holding bytes it could not name and no scope to complain in.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequest<'a> {
    /// Tag `0`. Create an agent under a name.
    AgentsCreate(agents::create::client::request::Frame),
    /// Tag `1`. Delete an agent by name.
    AgentsDelete(agents::delete::client::request::Frame),
    /// Tag `2`. Send an agent a message.
    AgentsMessage(agents::message::client::request::Frame),
    /// Tag `3`. Read an agent's log, filtered, and perhaps kept open.
    AgentsLogs(agents::logs::client::request::Frame),
    /// Tag `4`. List the caller's agents.
    AgentsList(agents::list::client::request::Frame),
    /// Tag `5`. Change what an agent mounts.
    AgentsEdit(agents::edit::client::request::Frame),
    /// Tag `6`. Create a tool under a name.
    ToolsCreate(tools::create::client::request::Frame),
    /// Tag `7`. Change what a tool mounts.
    ToolsEdit(tools::edit::client::request::Frame),
    /// Tag `8`. Hold somebody else's tool container under a name.
    ToolsConnect(tools::connect::client::request::Frame),
    /// Tag `9`. Attach a tool to an agent.
    ToolsAttach(tools::attach::client::request::Frame),
    /// Tag `10`. Detach a tool from an agent.
    ToolsDetach(tools::detach::client::request::Frame),
    /// Tag `11`. Delete a tool by name.
    ToolsDelete(tools::delete::client::request::Frame),
    /// Tag `12`. List the caller's tools.
    ToolsList(tools::list::client::request::Frame),
    /// Something this version cannot read, kept as it arrived.
    ///
    /// No tag of its own. It is not a request a client sends — it is
    /// what a request BECOMES when the daemon cannot make one of the
    /// others out of it, so it carries the payload verbatim and
    /// encodes back to exactly those bytes. The daemon finishes the
    /// scope over it with nothing in front: a finish that no response
    /// precedes is already what the wire means by a request that
    /// could not be served, and it names no endpoint's error
    /// vocabulary, which an invalid request has none of.
    Invalid(&'a [u8]),
}

impl Encode for ClientRequest<'_> {
    /// The ordinary JSON failure: every request is JSON after its
    /// tag.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            ClientRequest::AgentsCreate(frame) => frame.encode(out),
            ClientRequest::AgentsDelete(frame) => frame.encode(out),
            ClientRequest::AgentsMessage(frame) => frame.encode(out),
            ClientRequest::AgentsLogs(frame) => frame.encode(out),
            ClientRequest::AgentsList(frame) => frame.encode(out),
            ClientRequest::AgentsEdit(frame) => frame.encode(out),
            ClientRequest::ToolsCreate(frame) => frame.encode(out),
            ClientRequest::ToolsEdit(frame) => frame.encode(out),
            ClientRequest::ToolsConnect(frame) => frame.encode(out),
            ClientRequest::ToolsAttach(frame) => frame.encode(out),
            ClientRequest::ToolsDetach(frame) => frame.encode(out),
            ClientRequest::ToolsDelete(frame) => frame.encode(out),
            ClientRequest::ToolsList(frame) => frame.encode(out),
            ClientRequest::Invalid(bytes) => {
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

/// Dispatch on the tag, and fall back to
/// [`Invalid`](ClientRequest::Invalid) rather than refusing.
///
/// Each request is handed the WHOLE payload, tag included, because
/// each checks its own tag — this reads the byte to choose a decoder
/// and does not strip it.
impl<'a> Decode<'a> for ClientRequest<'a> {
    /// [`Infallible`]: a payload that cannot be read is a request all
    /// the same.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        let Some(tag) = bytes.first() else {
            return Ok(ClientRequest::Invalid(bytes));
        };
        let request = match *tag {
            0 => agents::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsCreate)
                .ok(),
            1 => agents::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDelete)
                .ok(),
            2 => agents::message::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsMessage)
                .ok(),
            3 => agents::logs::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsLogs)
                .ok(),
            4 => agents::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsList)
                .ok(),
            5 => agents::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsEdit)
                .ok(),
            6 => tools::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsCreate)
                .ok(),
            7 => tools::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsEdit)
                .ok(),
            8 => tools::connect::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsConnect)
                .ok(),
            9 => tools::attach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsAttach)
                .ok(),
            10 => tools::detach::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDetach)
                .ok(),
            11 => tools::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsDelete)
                .ok(),
            12 => tools::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsList)
                .ok(),
            _ => None,
        };
        Ok(request.unwrap_or(ClientRequest::Invalid(bytes)))
    }
}

impl fmt::Display for ClientRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientRequest::AgentsCreate(_) => f.write_str("agents create"),
            ClientRequest::AgentsDelete(_) => f.write_str("agents delete"),
            ClientRequest::AgentsMessage(_) => f.write_str("agents message"),
            ClientRequest::AgentsLogs(_) => f.write_str("agents logs"),
            ClientRequest::AgentsList(_) => f.write_str("agents list"),
            ClientRequest::AgentsEdit(_) => f.write_str("agents edit"),
            ClientRequest::ToolsCreate(_) => f.write_str("tools create"),
            ClientRequest::ToolsEdit(_) => f.write_str("tools edit"),
            ClientRequest::ToolsConnect(_) => f.write_str("tools connect"),
            ClientRequest::ToolsAttach(_) => f.write_str("tools attach"),
            ClientRequest::ToolsDetach(_) => f.write_str("tools detach"),
            ClientRequest::ToolsDelete(_) => f.write_str("tools delete"),
            ClientRequest::ToolsList(_) => f.write_str("tools list"),
            ClientRequest::Invalid(_) => f.write_str("an invalid request"),
        }
    }
}
