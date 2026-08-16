//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use super::{agentic_loop, images, laboratories, mcp_plugin, volumes};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a
/// [`Request`](crate::frame::client::ClientFrame::Request).
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
/// [`Invalid`](Self::Invalid) rather than a failure, so a server
/// always has a request to answer and a malformed one is a scope like
/// any other.
///
/// The alternative was refusing to decode, which would leave a server
/// holding bytes it could not name and no scope to complain in. The
/// only thing left would be closing the connection, which punishes a
/// client's whole session for one bad frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequest<'a> {
    /// Tag `0`. Run an agent and stream what it does.
    AgenticLoopRun(agentic_loop::run::client::request::Frame),
    /// Tag `1`. Run an MCP plugin.
    McpPluginRun(mcp_plugin::run::client::request::Frame),
    /// Tag `2`. Run a laboratory.
    LaboratoriesRun(laboratories::run::client::request::Frame),
    /// Tag `3`. Join one somebody else is running.
    LaboratoriesConnect(laboratories::connect::client::request::Frame<'a>),
    /// Tag `4`. List the volumes a provider offers.
    VolumesList(volumes::list::client::request::Frame),
    /// Tag `5`. Watch one of them.
    VolumesWatch(volumes::watch::client::request::Frame),
    /// Tag `6`. Make a volume.
    VolumesCreate(volumes::create::client::request::Frame),
    /// Tag `7`. Change how much one reserves.
    VolumesEdit(volumes::edit::client::request::Frame),
    /// Tag `8`. Destroy one.
    VolumesDelete(volumes::delete::client::request::Frame),
    /// Tag `9`. Ask whether an image can be supplied.
    ImagesCheck(images::check::client::request::Frame),
    /// Something this version cannot read, kept as it arrived.
    ///
    /// No tag of its own. It is not a request a client sends — it is
    /// what a request BECOMES when a server cannot make one of the
    /// others out of it, so it carries the payload verbatim and
    /// encodes back to exactly those bytes.
    ///
    /// # It is answered, not dropped
    ///
    /// A server acks it, minting a scope like any other, and responds
    /// with an error. Which is the whole point: a client that sent
    /// something wrong learns so, in the scope it asked for, and its
    /// other work carries on.
    ///
    /// # Why it carries no reason
    ///
    /// A server that wants one has the bytes and can ask the specific
    /// request to decode them, which answers precisely: an unknown
    /// tag, a body that would not parse, or nothing at all. Storing a
    /// reason here would mean this type choosing which of ten error
    /// vocabularies to speak, and choosing wrong for nine of them.
    Invalid(&'a [u8]),
}

impl Encode for ClientRequest<'_> {
    /// Three ways to fail, because ten requests use three encodings
    /// between them.
    type Error = ClientRequestEncodeError;

    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), ClientRequestEncodeError> {
        use ClientRequestEncodeError as E;
        match self {
            ClientRequest::AgenticLoopRun(frame) => {
                frame.encode(out).map_err(E::Json)
            }
            ClientRequest::McpPluginRun(frame) => {
                frame.encode(out).map_err(E::Json)
            }
            ClientRequest::LaboratoriesRun(frame) => {
                frame.encode(out).map_err(E::Json)
            }
            ClientRequest::LaboratoriesConnect(frame) => {
                frame.encode(out).map_err(E::LaboratoriesConnect)
            }
            ClientRequest::VolumesList(frame) => {
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                frame.encode(out).map_err(|error| match error {})
            }
            ClientRequest::VolumesWatch(frame) => {
                frame.encode(out).map_err(E::Postcard)
            }
            ClientRequest::VolumesCreate(frame) => {
                frame.encode(out).map_err(E::Postcard)
            }
            ClientRequest::VolumesEdit(frame) => {
                frame.encode(out).map_err(E::Postcard)
            }
            ClientRequest::VolumesDelete(frame) => {
                frame.encode(out).map_err(E::Postcard)
            }
            ClientRequest::ImagesCheck(frame) => {
                frame.encode(out).map_err(E::Json)
            }
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
            0 => agentic_loop::run::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgenticLoopRun)
                .ok(),
            1 => mcp_plugin::run::client::request::Frame::decode(bytes)
                .map(ClientRequest::McpPluginRun)
                .ok(),
            2 => laboratories::run::client::request::Frame::decode(bytes)
                .map(ClientRequest::LaboratoriesRun)
                .ok(),
            3 => laboratories::connect::client::request::Frame::decode(bytes)
                .map(ClientRequest::LaboratoriesConnect)
                .ok(),
            4 => volumes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesList)
                .ok(),
            5 => volumes::watch::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesWatch)
                .ok(),
            6 => volumes::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreate)
                .ok(),
            7 => volumes::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEdit)
                .ok(),
            8 => volumes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDelete)
                .ok(),
            9 => images::check::client::request::Frame::decode(bytes)
                .map(ClientRequest::ImagesCheck)
                .ok(),
            _ => None,
        };
        Ok(request.unwrap_or(ClientRequest::Invalid(bytes)))
    }
}

/// A request that could not be written.
///
/// Named for the encoding rather than for the request, because ten
/// requests share three of them and a variant per request would be
/// seven names that mean the same failure.
#[derive(Debug)]
pub enum ClientRequestEncodeError {
    /// A JSON request did not serialize.
    Json(serde_json::Error),
    /// A postcard request did not serialize.
    Postcard(postcard::Error),
    /// A laboratory connection did not serialize.
    ///
    /// Its own type, because its layout is hand-rolled and its one
    /// failure — an id too long for a `u16` — is not something a
    /// format reports.
    LaboratoriesConnect(
        laboratories::connect::client::request::FrameEncodeError,
    ),
}

impl fmt::Display for ClientRequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientRequestEncodeError::Json(error) => {
                write!(f, "request did not serialize as json: {error}")
            }
            ClientRequestEncodeError::Postcard(error) => {
                write!(f, "request did not serialize as postcard: {error}")
            }
            ClientRequestEncodeError::LaboratoriesConnect(error) => {
                write!(f, "connection request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for ClientRequestEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientRequestEncodeError::Json(error) => Some(error),
            ClientRequestEncodeError::Postcard(error) => Some(error),
            ClientRequestEncodeError::LaboratoriesConnect(error) => {
                Some(error)
            }
        }
    }
}
