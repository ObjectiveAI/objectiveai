//! One entry of a listed directory.

use serde::{Deserialize, Serialize};

use super::{ResponseEncodeError, ResponseError, prefixed};
use crate::encode::Writer;

/// What an entry is — and, on a
/// [`mount`](crate::container_proxy::fuse::mount) request, which kind
/// of mount: one file, or a tree. One byte on the binary answers;
/// `"file"` or `"directory"` where it rides JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A regular file. Kind `0`.
    File,
    /// A directory. Kind `1`.
    Directory,
}

/// Kind byte for a file.
const FILE: u8 = 0;

/// Kind byte for a directory.
const DIRECTORY: u8 = 1;

impl Kind {
    /// The one byte that says which.
    pub(crate) fn byte(self) -> u8 {
        match self {
            Kind::File => FILE,
            Kind::Directory => DIRECTORY,
        }
    }

    /// Which, from its byte.
    pub(crate) fn from_byte(byte: u8) -> Result<Self, ResponseError> {
        match byte {
            FILE => Ok(Kind::File),
            DIRECTORY => Ok(Kind::Directory),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}

/// One entry of a directory, as a [`list`](super::list) answers it.
///
/// ```text
/// [kind: u8][name_len: u16 BE][name: utf8…]
/// ```
///
/// The name is the entry's own, one path component, never `.` or
/// `..`. What a listing says is what a `readdir` needs and no more;
/// an entry's size is a [`stat`](super::stat) of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entry<'a> {
    /// The entry's name.
    pub name: &'a str,
    /// File or directory.
    pub kind: Kind,
}

impl Entry<'_> {
    /// Write one entry.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) -> Result<(), ResponseEncodeError> {
        out.extend_from_slice(&[self.kind.byte()]);
        prefixed::put(out, self.name.as_bytes()).map_err(ResponseEncodeError::NameLength)
    }
}

impl<'a> Entry<'a> {
    /// Split one entry off the front: the entry, then the rest.
    pub(crate) fn decode(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Truncated)?;
        let kind = Kind::from_byte(*kind)?;
        let (name, rest) = prefixed::take(rest).map_err(|_| ResponseError::Truncated)?;
        Ok((
            Entry {
                name: std::str::from_utf8(name).map_err(|_| ResponseError::NameUtf8)?,
                kind,
            },
            rest,
        ))
    }
}
