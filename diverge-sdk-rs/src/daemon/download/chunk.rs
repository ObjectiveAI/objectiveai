//! One piece of one file, and which file.

use serde::ser::SerializeStruct as _;
use serde::{Serialize, Serializer};
use std::fmt;

use serde::ser::Error as _;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One piece of one file in a download, with the file's path.
///
/// # The path is relative to what was asked for
///
/// A download names a file or a directory. The chunks of a file carry
/// the empty path, `[]`: the file is what was asked for, and there is
/// nothing to add to the destination. The chunks of a file in a
/// directory carry its path within that directory, as components, so a
/// directory `src` holding `a/b.txt` sends chunks with the path
/// `["a","b.txt"]`. Every chunk carries its path, the file's chunks are
/// consecutive and in order, and a reader that writes each chunk at its
/// destination joined with the path holds no state but the one file it
/// is writing. Files come in bytewise order of their paths. A zero-byte
/// file is exactly one chunk with an empty body, so that a file is
/// never sent as nothing. A directory with no file in it is not sent: a
/// download is files.
///
/// # On the wire
///
/// A `u32`, big-endian, then that many bytes of JSON — the path as an
/// array of strings — then the body, at most
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE), raw to the end of the payload.
/// The body is the read's own bytes, as a
/// [`read`](crate::shared::containers::read) carries them: no length of
/// its own, since the payload's end is the body's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chunk<'a> {
    /// The file, relative to what was asked for: empty for the file
    /// itself, its path within the directory otherwise. Each component
    /// a name: not empty, not `.` or `..`, holding no `/` and no NUL.
    pub path: Vec<String>,
    /// The piece: at most [`CHUNK_SIZE`](crate::CHUNK_SIZE) bytes,
    /// appended to the pieces of the same file before it. Empty for a
    /// zero-byte file's one chunk.
    pub body: &'a [u8],
}

impl Encode for Chunk<'_> {
    /// The path's JSON failure, or a path whose JSON is longer than a
    /// `u32` counts, which no path is.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        let path = serde_json::to_vec(&self.path)?;
        let length = u32::try_from(path.len()).map_err(|_| serde_json::Error::custom("path longer than a u32 counts"))?;
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&path);
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Chunk<'a> {
    /// Two ways to fail, and only one of them is JSON.
    type Error = ChunkError;

    fn decode(bytes: &'a [u8]) -> Result<Self, ChunkError> {
        let head: [u8; 4] = bytes.get(..4).and_then(|head| head.try_into().ok()).ok_or(ChunkError::Short)?;
        let rest = &bytes[4..];
        let length = usize::try_from(u32::from_be_bytes(head)).map_err(|_| ChunkError::Short)?;
        let path = rest.get(..length).ok_or(ChunkError::Short)?;
        let path = serde_json::from_slice(path).map_err(ChunkError::Path)?;
        Ok(Chunk { path, body: &rest[length..] })
    }
}

/// A chunk that could not be read.
#[derive(Debug)]
pub enum ChunkError {
    /// Fewer bytes than the length prefix, or than the path it counts.
    Short,
    /// The path did not parse as a JSON array of strings.
    Path(serde_json::Error),
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::Short => f.write_str("download chunk is shorter than its path"),
            ChunkError::Path(error) => write!(f, "download chunk path did not parse: {error}"),
        }
    }
}

impl std::error::Error for ChunkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChunkError::Path(error) => Some(error),
            ChunkError::Short => None,
        }
    }
}

/// As JSON, for a client that prints what the daemon answered: the
/// path, and the body's size as `bytes` — the body itself is not
/// JSON's to carry, and a client that wants it takes the chunk off the
/// wire.
impl Serialize for Chunk<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut chunk = serializer.serialize_struct("Chunk", 2)?;
        chunk.serialize_field("path", &self.path)?;
        chunk.serialize_field("bytes", &self.body.len())?;
        chunk.end()
    }
}
