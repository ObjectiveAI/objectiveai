//! A resource the run rewrote, surfaced whole under its name.

use std::error;
use std::fmt;
use std::str::Utf8Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A resource, rewritten by the run and handed back to the caller:
/// the state the request supplied by identity, in its new form.
///
/// A `*_resource` field names caller-held state the run MUTATES —
/// an OAuth entry whose tokens rotate, a token file the CLI
/// rewrites. This is how the mutation gets home: the provider
/// surfaces the resource's whole new content, and the caller
/// REPLACES what it holds, so the next request names the new
/// identity. A resource frame is not terminal: it may come whenever
/// the provider observes a change, and may repeat — the last one
/// wins.
///
/// # The name is the field
///
/// The name is the path of the request field that supplied the
/// resource, dotted, exactly as the request spells it:
/// `provider.auth_resource`, `toolsets.spotify.auth_resource`. The
/// caller matched it once on the way in and matches it the same
/// way on the way back; no identity is needed, because the content
/// IS the new identity's preimage.
///
/// # Wire form
///
/// `[u32 BE: byte length of the name][name, UTF-8][body, verbatim]`
/// — the whole resource in one frame, never chunked: a resource is
/// a state document, and it fits. Both halves borrow from the frame
/// they arrived in, the fetch frames' way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resource<'a> {
    /// The request field's dotted path.
    pub name: &'a str,
    /// The resource's new content, whole.
    pub body: &'a [u8],
}

/// The bytes the name length occupies.
const NAME_LEN: usize = 4;

impl Encode for Resource<'_> {
    /// The one thing that can fail: a name too long for four bytes
    /// to measure. Carried rather than truncated — a length that
    /// lies is a frame that cannot be read.
    type Error = ResourceEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), ResourceEncodeError> {
        let len = u32::try_from(self.name.len())
            .map_err(|_| ResourceEncodeError::NameLength(self.name.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(self.name.as_bytes());
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Resource<'a> {
    /// Three ways to fail, all in the name: too short to hold its
    /// length, a length the frame does not reach, or bytes that are
    /// not UTF-8. The body is bytes taken as bytes.
    type Error = ResourceError;

    fn decode(bytes: &'a [u8]) -> Result<Self, ResourceError> {
        if bytes.len() < NAME_LEN {
            return Err(ResourceError::Short(bytes.len()));
        }
        let (len, rest) = bytes.split_at(NAME_LEN);
        let len = u32::from_be_bytes(
            <[u8; NAME_LEN]>::try_from(len).expect("split_at gave 4 bytes"),
        ) as usize;
        if rest.len() < len {
            return Err(ResourceError::Name {
                need: len,
                have: rest.len(),
            });
        }
        let (name, body) = rest.split_at(len);
        Ok(Resource {
            name: std::str::from_utf8(name).map_err(ResourceError::NameUtf8)?,
            body,
        })
    }
}

/// A resource frame that could not be written.
#[derive(Debug)]
pub enum ResourceEncodeError {
    /// The name is longer than the four-byte length can say.
    NameLength(usize),
}

impl fmt::Display for ResourceEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceEncodeError::NameLength(len) => {
                write!(f, "resource name is {len} bytes, more than u32 can say")
            }
        }
    }
}

impl error::Error for ResourceEncodeError {}

/// A resource frame that could not be read.
#[derive(Debug)]
pub enum ResourceError {
    /// Fewer bytes than the name length itself occupies.
    Short(usize),
    /// The name length says more bytes than the frame holds.
    Name {
        /// What the length asked for.
        need: usize,
        /// What was there.
        have: usize,
    },
    /// The name is not UTF-8.
    NameUtf8(Utf8Error),
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::Short(len) => write!(
                f,
                "resource frame is {len} bytes, too short for a name length"
            ),
            ResourceError::Name { need, have } => write!(
                f,
                "resource name length says {need} bytes but {have} remain"
            ),
            ResourceError::NameUtf8(error) => {
                write!(f, "resource name is not UTF-8: {error}")
            }
        }
    }
}

impl error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ResourceError::NameUtf8(error) => Some(error),
            ResourceError::Short(_) | ResourceError::Name { .. } => None,
        }
    }
}
