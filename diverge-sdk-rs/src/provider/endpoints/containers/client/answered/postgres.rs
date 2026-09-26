//! The caller's half of a database connection: what the container
//! wrote, verbatim.

use std::convert::Infallible;

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::postgres;

/// The bytes the container wrote, one piece per frame, never parsed;
/// the finish is the container's socket ended. Nothing refuses.
#[derive(Debug, Clone, Copy)]
pub struct Postgres;

impl Answered for Postgres {
    type Item = Bytes;
    type Error = Infallible;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Bytes, Infallible>, Infallible> {
        let frame = postgres::response::Frame::decode(&payload)?;
        Ok(Ok(payload.slice_ref(frame.0)))
    }
}
