//! Opening the scope and streaming the files out.

use serde::ser::SerializeMap as _;
use serde::{Serialize, Serializer};
use std::borrow::Cow;

use bytes::Bytes;
use schemars::{JsonSchema, Schema, SchemaGenerator};

use super::super::request;
use super::super::super::server::response;
use crate::daemon::client::stream;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::daemon::download;
use crate::shared::error::Error;

/// A download that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream: one [`Item`] per response, ended by the daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<Item, response::FrameError>;

/// One response of the download, owned: the [`response::Frame`] with
/// its chunk's body copied out of the frame it arrived in, so that the
/// item outlives the message.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// One piece of one file, with the file's path relative to what was
    /// asked for: see [`Chunk`](crate::daemon::download::Chunk).
    Chunk {
        /// The file, relative to what was asked for.
        path: Vec<String>,
        /// The piece, appended to the pieces of the same file before
        /// it.
        body: Bytes,
    },
    /// No agent is the one named, or nothing is at the path.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it.
    Forbidden,
    /// The daemon's failure.
    Error(Error),
}

/// Open the download and hand back its stream, without reading
/// anything: the daemon's first frame is the stream's. A finish with
/// nothing before it is a directory with no file.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<ExecuteStream, OpenError> {
    stream::execute(handle, request, decode).await
}

/// One response, read as an [`Item`], the chunk's body sliced out of
/// the message it arrived in.
fn decode(bytes: &Bytes, payload: &[u8]) -> Result<Item, response::FrameError> {
    Ok(match response::Frame::decode(payload)? {
        response::Frame::Chunk(chunk) => Item::Chunk { path: chunk.path, body: bytes.slice_ref(chunk.body) },
        response::Frame::NotFound => Item::NotFound,
        response::Frame::Forbidden => Item::Forbidden,
        response::Frame::Error(error) => Item::Error(error),
    })
}

/// As JSON, as the [`response::Frame`] is: the variant, snake case,
/// over its payload, a chunk by its path and its size.
/// The schema of what `Serialize` writes: see
/// [`download::answer_schema`].
impl JsonSchema for Item {
    fn schema_name() -> Cow<'static, str> {
        "AgentsDownloadItem".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "diverge_sdk::daemon::endpoints::agents::download::client::execute::Item".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        download::answer_schema(generator, false)
    }
}

impl Serialize for Item {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Item::Chunk { path, body } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("chunk", &Piece { path, bytes: body.len() })?;
                map.end()
            }
            Item::NotFound => serializer.serialize_str("not_found"),
            Item::Forbidden => serializer.serialize_str("forbidden"),
            Item::Error(error) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("error", error)?;
                map.end()
            }
        }
    }
}

/// A chunk as JSON: its path and its size.
#[derive(Serialize)]
struct Piece<'a> {
    path: &'a [String],
    bytes: usize,
}
