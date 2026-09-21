//! The begin scope, held for the connection's life.

use rmcp::model::ContentBlock;
use serde_json::Value;

use super::super::channel_request;
use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};
use crate::endpoints::containers::client::answered::{Dequeue, Enqueue, Postgres, Schema};
use crate::endpoints::containers::client::{ChannelStream, OpenError, UnaryError, unary};
use crate::shared::containers::{dequeue, enqueue, postgres};

/// The begin scope an agent container's server holds.
///
/// Every channel the server may open on it is a method here — the
/// arguments' schema, the queue's two verbs, the server's half of a
/// database connection — and so is answering a channel the proxy
/// opened, by the proxy's own channel number. Clones share the scope.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame. The scope is the connection's life,
/// and the connection ends when the container does.
#[derive(Debug, Clone)]
pub struct ExecuteHandle {
    handle: Handle,
    scope: u32,
}

/// One channel request of this family, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// What the arguments may be: the image's JSON Schema for them.
    pub async fn schema(&self) -> Result<Value, UnaryError<Schema>> {
        let payload = payload(&channel_request::Frame::Schema).map_err(UnaryError::Request)?;
        unary::<Schema>(&self.handle, self.scope, &payload).await
    }

    /// A message for the agent — starting a loop when none runs,
    /// queued when one does — answered with its fate whenever that
    /// is known; nothing times it out.
    pub async fn enqueue(&self, content: Vec<ContentBlock>) -> Result<enqueue::response::Frame, UnaryError<Enqueue>> {
        let payload = payload(&channel_request::Frame::Enqueue(enqueue::request::Request { content }))
            .map_err(UnaryError::Request)?;
        unary::<Enqueue>(&self.handle, self.scope, &payload).await
    }

    /// Clear the agent's queue: whether it held anything.
    pub async fn dequeue(&self) -> Result<dequeue::response::Frame, UnaryError<Dequeue>> {
        let payload = payload(&channel_request::Frame::Dequeue).map_err(UnaryError::Request)?;
        unary::<Dequeue>(&self.handle, self.scope, &payload).await
    }

    /// The server's half of a database connection the proxy
    /// announced, quoting the id the proxy minted: what comes back is
    /// everything the container's driver wrote, until the finish.
    pub async fn postgres(&self, connection_id: u32) -> Result<ChannelStream<Postgres>, OpenError> {
        let payload = payload(&channel_request::Frame::Postgres(postgres::request::Postgres { connection_id }))
            .map_err(OpenError::Request)?;
        let channel = self
            .handle
            .send_channel_request(self.scope, &payload)
            .await
            .map_err(OpenError::Send)?;
        Ok(ChannelStream::new(channel.response_receiver))
    }

    /// One answer on a channel the proxy opened, by its number.
    pub async fn respond(&self, channel: u32, payload: &[u8]) -> Result<(), SendError> {
        self.handle.send_channel_response(self.scope, channel, payload).await
    }

    /// The end of the answers on a channel the proxy opened.
    pub async fn finish(&self, channel: u32) -> Result<(), SendError> {
        self.handle.send_channel_response_finish(self.scope, channel).await
    }
}
