//! What the arguments may be.

use bytes::Bytes;
use serde_json::Value;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::schema;
use crate::shared::error::Error;

/// The arguments' JSON Schema, once; or the image's refusal to state
/// one.
#[derive(Debug, Clone, Copy)]
pub struct Schema;

impl Answered for Schema {
    type Item = Value;
    type Error = schema::response::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Value, Error>, Self::Error> {
        Ok(match schema::response::Frame::decode(&payload)? {
            schema::response::Frame::Schema(schema) => Ok(schema),
            schema::response::Frame::Error(error) => Err(error),
        })
    }
}
