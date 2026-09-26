//! The frames a family encodes for the shared machinery.

use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// Three encoders a scope's `execute` hands the shared serving loop,
/// because each family's channel frames are a type of its own, byte-
/// identical to its siblings' but not the same type.
///
/// Every function answers `None` when the frame would not encode,
/// which the machinery treats as an answer it cannot give: a Postgres
/// dial declined, a write ended without its content.
#[derive(Debug, Clone, Copy)]
pub struct Encoders {
    /// The caller's half of a database connection: the family's
    /// `Postgres { connection_id }` channel request.
    pub postgres_half: fn(u32) -> Option<Vec<u8>>,
    /// One piece of a write's content: the family's `write_bytes`
    /// `Body` channel response.
    pub write_body: fn(&[u8]) -> Option<Vec<u8>>,
    /// A write's content ending in failure: the family's `write_bytes`
    /// `Error` channel response.
    pub write_error: fn(&Error) -> Option<Vec<u8>>,
}

/// One frame's bytes, or nothing for one that would not encode.
pub(crate) fn encoded<T: Encode>(frame: &T) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes)).ok()?;
    Some(bytes)
}
