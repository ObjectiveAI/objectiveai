//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};

use super::{agents, volumes};

/// The payload of a
/// [`Request`](diverge_provider_sdk::frame::client::ClientFrame::Request).
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
    /// Tag `5`. List the daemon's volumes.
    VolumesList(volumes::list::client::request::Frame),
    /// Tag `6`. Examine one of them.
    VolumesStat(volumes::stat::client::request::Frame),
    /// Tag `7`. Read one file out of one.
    VolumesRead(volumes::read::client::request::Frame),
    /// Tag `8`. Write one file into one.
    VolumesWrite(volumes::write::client::request::Frame),
    /// Tag `9`. See what one holds.
    VolumesFiletree(volumes::filetree::client::request::Frame),
    /// Tag `10`. Ask how large a volume may be made.
    VolumesCreateCapacity(volumes::create_capacity::client::request::Frame),
    /// Tag `11`. Make a volume.
    VolumesCreate(volumes::create::client::request::Frame),
    /// Tag `12`. Ask how far one may grow.
    VolumesEditCapacity(volumes::edit_capacity::client::request::Frame),
    /// Tag `13`. Change how much one reserves, or whether it keeps what is written into it.
    VolumesEdit(volumes::edit::client::request::Frame),
    /// Tag `14`. Destroy one.
    VolumesDelete(volumes::delete::client::request::Frame),
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
    /// Two ways to fail, because the requests use two encodings
    /// between them: the agents family is JSON after its tag, and the
    /// volumes family is postcard, as the provider's is — and two of
    /// the volumes requests use neither, having nothing to encode.
    type Error = ClientRequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), ClientRequestEncodeError> {
        match self {
            ClientRequest::AgentsCreate(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Json),
            ClientRequest::AgentsDelete(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Json),
            ClientRequest::AgentsMessage(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Json),
            ClientRequest::AgentsLogs(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Json),
            ClientRequest::AgentsList(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Json),
            ClientRequest::VolumesList(frame) => frame.encode(out).map_err(|error| match error {}),
            ClientRequest::VolumesStat(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesRead(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesWrite(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesFiletree(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesCreateCapacity(frame) => frame.encode(out).map_err(|error| match error {}),
            ClientRequest::VolumesCreate(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesEditCapacity(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesEdit(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
            ClientRequest::VolumesDelete(frame) => frame.encode(out).map_err(ClientRequestEncodeError::Postcard),
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
            5 => volumes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesList)
                .ok(),
            6 => volumes::stat::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesStat)
                .ok(),
            7 => volumes::read::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesRead)
                .ok(),
            8 => volumes::write::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesWrite)
                .ok(),
            9 => volumes::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesFiletree)
                .ok(),
            10 => volumes::create_capacity::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreateCapacity)
                .ok(),
            11 => volumes::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreate)
                .ok(),
            12 => volumes::edit_capacity::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEditCapacity)
                .ok(),
            13 => volumes::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEdit)
                .ok(),
            14 => volumes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDelete)
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
            ClientRequest::VolumesList(_) => f.write_str("volumes list"),
            ClientRequest::VolumesStat(_) => f.write_str("volumes stat"),
            ClientRequest::VolumesRead(_) => f.write_str("volumes read"),
            ClientRequest::VolumesWrite(_) => f.write_str("volumes write"),
            ClientRequest::VolumesFiletree(_) => f.write_str("volumes filetree"),
            ClientRequest::VolumesCreateCapacity(_) => f.write_str("volumes create capacity"),
            ClientRequest::VolumesCreate(_) => f.write_str("volumes create"),
            ClientRequest::VolumesEditCapacity(_) => f.write_str("volumes edit capacity"),
            ClientRequest::VolumesEdit(_) => f.write_str("volumes edit"),
            ClientRequest::VolumesDelete(_) => f.write_str("volumes delete"),
            ClientRequest::Invalid(_) => f.write_str("an invalid request"),
        }
    }
}

/// A request that would not serialize, by which library refused it.
#[derive(Debug)]
pub enum ClientRequestEncodeError {
    /// A JSON request that would not serialize.
    Json(serde_json::Error),
    /// A postcard request that would not serialize.
    Postcard(postcard::Error),
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
        }
    }
}

impl std::error::Error for ClientRequestEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientRequestEncodeError::Json(error) => Some(error),
            ClientRequestEncodeError::Postcard(error) => Some(error),
        }
    }
}
