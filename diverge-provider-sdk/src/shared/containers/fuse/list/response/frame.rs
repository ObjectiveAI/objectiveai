//! The answer to a listing.

use super::super::super::{Entry, ResponseEncodeError, ResponseError};
use crate::encode::{Encode, Writer};

/// The one message that answers a listing.
///
/// ```text
/// [kind: u8][count: u32 BE][entry…]… | [kind: u8][message…]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. The directory's entries, each an [`Entry`], in
    /// whatever order the caller keeps them. Empty is an empty
    /// directory.
    Entries(Vec<Entry<'a>>),
    /// Kind `1`. There is no such directory: the caller holds nothing
    /// at the path, or a file there.
    Missing,
    /// Kind `2`. The listing was refused or failed, and this says why,
    /// for a reader rather than a program.
    Error(&'a str),
}

/// The bytes the count occupies.
const COUNT: usize = 4;

impl Encode for Frame<'_> {
    /// One way to fail: an entry's name longer than its prefix holds.
    type Error = ResponseEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), ResponseEncodeError> {
        match self {
            Frame::Entries(entries) => {
                out.extend_from_slice(&[0]);
                // A listing of more than four billion entries is not a
                // message; the count saturates rather than wraps, and
                // the entries after it are simply not written.
                let count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
                out.extend_from_slice(&count.to_be_bytes());
                for entry in entries.iter().take(count as usize) {
                    entry.encode(out)?;
                }
            }
            Frame::Missing => out.extend_from_slice(&[1]),
            Frame::Error(message) => {
                out.extend_from_slice(&[2]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode the one message. The entries' names, or the message,
    /// borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => {
                let count: &[u8; COUNT] = rest
                    .get(..COUNT)
                    .and_then(|head| head.try_into().ok())
                    .ok_or(ResponseError::Truncated)?;
                let count = u32::from_be_bytes(*count) as usize;
                let mut rest = &rest[COUNT..];
                let mut entries = Vec::with_capacity(count.min(rest.len() / 11));
                for _ in 0..count {
                    let (entry, after) = Entry::decode(rest)?;
                    entries.push(entry);
                    rest = after;
                }
                Ok(Frame::Entries(entries))
            }
            1 => Ok(Frame::Missing),
            2 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
