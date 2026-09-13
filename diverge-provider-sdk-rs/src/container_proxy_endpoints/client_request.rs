//! Every request the server can open a scope with.

use std::convert::Infallible;

use super::{agents, filesystem, fuse, tools};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a
/// [`Request`](crate::frame::client::ClientFrame::Request).
///
/// One variant per scope, in tag order, plus
/// [`Invalid`](Self::Invalid). This is the only place the tag values
/// meet: each request states its own in its own module, and nothing
/// else can see all of them at once — so this is what the table in
/// [`container_proxy_endpoints`](super) is a table OF.
///
/// # Decoding cannot fail
///
/// There is no error type. A payload this does not understand — an
/// unknown tag, a body that will not parse, no bytes at all — becomes
/// [`Invalid`](Self::Invalid) rather than a failure, so the proxy
/// always has a request to answer and a malformed one is a scope like
/// any other.
///
/// The alternative was refusing to decode, which would leave the proxy
/// holding bytes it could not name and no scope to complain in. The
/// only thing left would be closing the connection, which is the
/// container's whole life over one bad frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequest<'a> {
    /// Tag `0`. Begin the server's work on an agent container.
    AgentsBegin(agents::begin::client::request::Frame),
    /// Tag `1`. Begin the server's work on a tool container.
    ToolsBegin(tools::begin::client::request::Frame),
    /// Tag `2`. Make one FUSE mount.
    FuseMount(fuse::mount::client::request::Frame),
    /// Tag `3`. Watch the container's tree.
    FilesystemTree(filesystem::tree::client::request::Frame),
    /// Tag `4`. Read one file out of the container.
    FilesystemRead(filesystem::read::client::request::Frame),
    /// Tag `5`. Write one file into the container.
    FilesystemWrite(filesystem::write::client::request::Frame),
    /// Something this version cannot read, kept as it arrived.
    ///
    /// No tag of its own. It is not a request the server sends — it is
    /// what a request BECOMES when the proxy cannot make one of the
    /// others out of it, so it carries the payload verbatim and
    /// encodes back to exactly those bytes.
    ///
    /// # It is answered, not dropped
    ///
    /// The proxy finishes the scope over it, with nothing in front:
    /// six scopes have six error vocabularies, and an invalid request
    /// names none of them — where a finish with nothing before it is
    /// already what the wire means by a request that could not be
    /// served. Which is the point all the same: a server that sent
    /// something wrong learns so, in the scope it opened, and its
    /// other work carries on.
    ///
    /// # Why it carries no reason
    ///
    /// The proxy has the bytes and can ask the specific request to
    /// decode them, which answers precisely: an unknown tag, a body
    /// that would not parse, or nothing at all. Storing a reason here
    /// would mean this type choosing which of six error vocabularies
    /// to speak, and choosing wrong for five of them.
    Invalid(&'a [u8]),
}

impl Encode for ClientRequest<'_> {
    /// The ordinary JSON failure: five of the six requests are JSON
    /// after their tag, and the sixth has nothing to encode.
    type Error = serde_json::Error;

    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            ClientRequest::AgentsBegin(frame) => frame.encode(out),
            ClientRequest::ToolsBegin(frame) => {
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                frame.encode(out).map_err(|error| match error {})
            }
            ClientRequest::FuseMount(frame) => frame.encode(out),
            ClientRequest::FilesystemTree(frame) => frame.encode(out),
            ClientRequest::FilesystemRead(frame) => frame.encode(out),
            ClientRequest::FilesystemWrite(frame) => frame.encode(out),
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
            0 => agents::begin::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsBegin)
                .ok(),
            1 => tools::begin::client::request::Frame::decode(bytes)
                .map(ClientRequest::ToolsBegin)
                .ok(),
            2 => fuse::mount::client::request::Frame::decode(bytes)
                .map(ClientRequest::FuseMount)
                .ok(),
            3 => filesystem::tree::client::request::Frame::decode(bytes)
                .map(ClientRequest::FilesystemTree)
                .ok(),
            4 => filesystem::read::client::request::Frame::decode(bytes)
                .map(ClientRequest::FilesystemRead)
                .ok(),
            5 => filesystem::write::client::request::Frame::decode(bytes)
                .map(ClientRequest::FilesystemWrite)
                .ok(),
            _ => None,
        };
        Ok(request.unwrap_or(ClientRequest::Invalid(bytes)))
    }
}
