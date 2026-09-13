//! A running agent container, and everything a caller can do with
//! it.

use std::fmt;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::Stream;
use serde_json::Value;

use super::super::channel_request;
use super::{Filetree, FiletreeStream, Read, ReadStream, WritePath};
use crate::encode::{Encode, Writer};
use crate::endpoints::containers::agents::run::server;
use crate::endpoints::containers::client::answered::{AgentSchema, Dequeue, Enqueue};
use crate::endpoints::containers::client::{OpenError, Scoped, UnaryError, WaitError};
use crate::shared::containers::{dequeue, enqueue, read, write_path};

/// The scope a run opened, held for the container's life.
///
/// Every channel a caller may open into an agent container is a
/// method here: the tree watched, a file read or written, the agent's
/// schema, the queue's two verbs, and the stop. What the agent says
/// is not: it is the [`ExecuteStream`](super::ExecuteStream) `execute` handed back
/// beside this. Each opens its own channel, so several may be in flight
/// at once. Clones share the scope, and [`wait`](Self::wait) on any
/// of them reports the same end.
///
/// # Dropping it does nothing
///
/// No destructor sends a frame: a destructor could not await, could
/// not report a failure, and could not be skipped when a caller
/// wanted the container to outlive the value. Ending a run is
/// [`stop`](Self::stop), or the connection going.
#[derive(Debug, Clone)]
pub struct ExecuteHandle(Arc<Scoped>);

/// One channel request of this family, as its bytes.
fn payload(frame: &channel_request::Frame) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes))?;
    Ok(bytes)
}

impl ExecuteHandle {
    pub(super) fn new(scoped: Arc<Scoped>) -> Self {
        ExecuteHandle(scoped)
    }

    /// The scope's number, as this end minted it.
    pub fn scope(&self) -> u32 {
        self.0.scope()
    }

    /// The run's end: `Ok` when the scope finishes as it should — a
    /// stop, or the container's own end — and the provider's error
    /// when it ended that way. Resolves once and answers the same way
    /// again after.
    ///
    /// # It reads nothing
    ///
    /// The main stream is the conversation, and the
    /// [`ExecuteStream`](super::ExecuteStream) `execute` handed back is its reader.
    /// This waits for that reader to reach the end, so that no chunk
    /// is lost to a caller who only wanted to know the run is over; a
    /// caller that wants the end and not the conversation drains the
    /// chunks.
    pub async fn wait(&self) -> Result<(), WaitError<server::response::FrameError>> {
        self.0.ended().await
    }

    /// Stop the container. Nothing answers on the channel this opens;
    /// what answers is the scope's own finish, on [`wait`](Self::wait).
    pub async fn stop(&self) -> Result<(), OpenError> {
        let payload = payload(&channel_request::Frame::Stop).map_err(OpenError::Request)?;
        self.0.bare(&payload).await.map_err(OpenError::Send)
    }

    /// Watch the container's tree: a snapshot, then every change.
    pub async fn filetree(&self) -> Result<FiletreeStream, OpenError> {
        let payload = payload(&channel_request::Frame::Filetree).map_err(OpenError::Request)?;
        self.0.open::<Filetree>(&payload).await.map_err(OpenError::Send)
    }

    /// Read one file out of the container, as path components from
    /// its root.
    pub async fn read(&self, path: Vec<String>) -> Result<ReadStream, OpenError> {
        let payload = payload(&channel_request::Frame::Read(read::request::Request { path }))
            .map_err(OpenError::Request)?;
        self.0.open::<Read>(&payload).await.map_err(OpenError::Send)
    }

    /// Write one file into the container, whole, from `content`'s
    /// pieces; resolves when it is at the path. A piece that fails
    /// ends the content with an error the provider sees, and the
    /// provider's own answer is what comes back.
    pub async fn write<S, E>(&self, path: Vec<String>, content: S) -> Result<(), UnaryError<WritePath>>
    where
        S: Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: fmt::Display + Send + 'static,
    {
        self.0
            .write::<WritePath, S, E>(
                move |write_id| {
                    payload(&channel_request::Frame::Write(write_path::request::Request {
                        write_id,
                        path,
                    }))
                },
                content,
            )
            .await
    }

    /// What the agent value may be: the image's JSON Schema for it.
    pub async fn agent_schema(&self) -> Result<Value, UnaryError<AgentSchema>> {
        let payload = payload(&channel_request::Frame::AgentSchema).map_err(UnaryError::Request)?;
        self.0.unary::<AgentSchema>(&payload).await
    }

    /// A message for the agent — starting a loop when none runs,
    /// queued when one does — answered with its fate whenever that
    /// is known, which may be long after the ask; nothing times it
    /// out. What the agent says arrives on the scope's main stream.
    pub async fn enqueue(&self, prompt: String) -> Result<enqueue::response::Frame, UnaryError<Enqueue>> {
        let payload = payload(&channel_request::Frame::Enqueue(enqueue::request::Request { prompt }))
            .map_err(UnaryError::Request)?;
        self.0.unary::<Enqueue>(&payload).await
    }

    /// Clear the running loop's queue: whether it held anything.
    pub async fn dequeue(&self) -> Result<dequeue::response::Frame, UnaryError<Dequeue>> {
        let payload = payload(&channel_request::Frame::Dequeue).map_err(UnaryError::Request)?;
        self.0.unary::<Dequeue>(&payload).await
    }
}
