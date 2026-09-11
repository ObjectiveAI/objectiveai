//! Asking for the schema.

use futures_util::StreamExt as _;
use serde_json::Value;

use super::super::response;
use super::ExecuteError;
use crate::decode::Decode as _;
use crate::server::container_client::{self, ContainerClient};
use crate::server::messages::{MessageError, Messages};

/// Open `/agent/schema` and read the one answer.
///
/// The schema, as the image states it; or
/// [`Refused`](ExecuteError::Refused) with the container's reason, an
/// image that states none above all.
pub async fn execute(client: &ContainerClient) -> Result<Value, ExecuteError> {
    let socket = client.open("/agent/schema").await.map_err(ExecuteError::Open)?;
    let mut messages = Messages::new(socket);
    let answer = match messages.next().await {
        None => return Err(ExecuteError::Unserved),
        Some(Err(MessageError::Socket(error))) => return Err(ExecuteError::Socket(error)),
        Some(Err(MessageError::Closed)) => return Err(ExecuteError::Closed),
        Some(Ok(bytes)) => bytes,
    };
    let frame = response::Frame::decode(&answer).map_err(ExecuteError::Frame)?;
    // The proxy closes after its one message; hear it out.
    if let Some(mut socket) = messages.into_inner() {
        let _ = container_client::drain(&mut socket).await;
    }
    match frame {
        response::Frame::AgentSchema(schema) => Ok(schema),
        response::Frame::Error(error) => Err(ExecuteError::Refused(error)),
    }
}
