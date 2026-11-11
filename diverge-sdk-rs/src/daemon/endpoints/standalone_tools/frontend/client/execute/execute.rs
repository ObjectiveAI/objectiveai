//! Opening the scope, its answer, and the way in.

use super::super::super::server::response;
use super::super::{channel_request, request};
use crate::daemon::client::{one_shot, stream};
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};

/// Why the scope did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The daemon's answers, in order, until the finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Send the request: its answer, and the handle that writes its body
/// and cancels it. Dropping the stream abandons the scope, which drops
/// the request too.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Frontend), OpenError> {
    let scope = one_shot::open::<request::Frame, ()>(handle, request).await.map_err(OpenError::from)?;
    let frontend = Frontend { handle: handle.clone(), scope: scope.scope };
    Ok((stream::ExecuteStream::new(scope.response_receiver, response::decode), frontend))
}

/// The way into an open frontend request: each call one channel request
/// on the scope.
#[derive(Debug, Clone)]
pub struct Frontend {
    handle: Handle,
    scope: u32,
}

impl Frontend {
    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Send a piece of the body; resolves once the daemon has taken it.
    /// The next is sent once this resolves, which keeps them in order.
    pub async fn body(&self, bytes: &[u8]) -> Result<(), SendError> {
        self.send(&channel_request::Frame::Body(bytes), true).await
    }

    /// End the body; resolves once the daemon has taken it.
    pub async fn end(&self) -> Result<(), SendError> {
        self.send(&channel_request::Frame::End, true).await
    }

    /// Drop the request. Returns once the request is written; the
    /// scope's finish, which is the daemon's answer to it, arrives on
    /// the answer.
    pub async fn cancel(&self) -> Result<(), SendError> {
        self.send(&channel_request::Frame::Cancel, false).await
    }

    async fn send(&self, frame: &channel_request::Frame<'_>, wait: bool) -> Result<(), SendError> {
        let mut payload = Vec::new();
        // Nothing can go wrong.
        let _ = frame.encode(&mut Writer::new(&mut payload));
        let mut channel = self.handle.send_channel_request(self.scope, &payload).await?;
        if wait {
            while channel.response_receiver.recv().await.is_some() {}
        }
        Ok(())
    }
}
