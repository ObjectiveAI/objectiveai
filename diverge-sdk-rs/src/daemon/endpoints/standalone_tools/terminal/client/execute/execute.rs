//! Opening the scope, its output, and the way in.

use super::super::super::server::response;
use super::super::{channel_request, request};
use crate::daemon::client::{one_shot, stream};
use crate::shared::containers::terminal::open::Open;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::encode::{Encode, Writer};

/// Why the scope did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The daemon's answers, in order, until the finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Open the terminal: its output, and the handle that writes its
/// stdin, resizes it and closes it. Dropping the stream abandons the
/// scope, which kills the shell too.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Terminal), OpenError> {
    let scope = one_shot::open::<request::Frame, ()>(handle, request).await.map_err(OpenError::from)?;
    let terminal = Terminal { handle: handle.clone(), scope: scope.scope };
    Ok((stream::ExecuteStream::new(scope.response_receiver, response::decode), terminal))
}

/// The way into an open terminal: each call one channel request on the
/// scope, resolving once the daemon has finished it.
#[derive(Debug, Clone)]
pub struct Terminal {
    handle: Handle,
    scope: u32,
}

impl Terminal {
    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Write `bytes` to the shell's stdin; resolves once written. The
    /// next is sent once this resolves, which keeps them in order.
    pub async fn stdin(&self, bytes: &[u8]) -> Result<(), SendError> {
        self.send(&channel_request::Frame::Stdin(bytes), true).await
    }

    /// Set the terminal's size; resolves once set.
    pub async fn resize(&self, size: Open) -> Result<(), SendError> {
        self.send(&channel_request::Frame::Resize(size), true).await
    }

    /// Kill the shell. Returns once the request is written; the scope's
    /// finish, which is the daemon's answer to it, arrives on the
    /// output.
    pub async fn close(&self) -> Result<(), SendError> {
        self.send(&channel_request::Frame::Close, false).await
    }

    async fn send(&self, frame: &channel_request::Frame<'_>, wait: bool) -> Result<(), SendError> {
        let mut payload = Vec::new();
        // A size always serializes.
        let _ = frame.encode(&mut Writer::new(&mut payload));
        let mut channel = self.handle.send_channel_request(self.scope, &payload).await?;
        if wait {
            while channel.response_receiver.recv().await.is_some() {}
        }
        Ok(())
    }
}
