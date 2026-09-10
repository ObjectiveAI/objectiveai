//! What the agent value may be.

use bytes::Bytes;
use serde_json::Value;

use super::Answered;
use crate::decode::Decode as _;
use crate::shared::containers::agent_schema;
use crate::shared::error::Error;

/// The agent's JSON Schema, once; or the image's refusal to state
/// one.
#[derive(Debug, Clone, Copy)]
pub struct AgentSchema;

impl Answered for AgentSchema {
    type Item = Value;
    type Error = agent_schema::response::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Value, Error>, Self::Error> {
        Ok(match agent_schema::response::Frame::decode(&payload)? {
            agent_schema::response::Frame::AgentSchema(schema) => Ok(schema),
            agent_schema::response::Frame::Error(error) => Err(error),
        })
    }
}
