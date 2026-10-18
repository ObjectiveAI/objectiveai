//! Ending a scope that would not end on its own.

use crate::wire::client::handle::{Handle, SendError};

/// The one byte every cancellable scope's cancel is: the tag of the
/// scope's one channel request, which carries nothing.
const CANCEL: [u8; 1] = [0];

/// A scope's cancel, held beside its stream or its pending answer: a
/// logs watch, a container's filetree, a message not yet taken. One
/// channel request, carrying nothing, on the scope; the daemon answers
/// nothing on the channel and finishes the scope, and what was sent
/// before the finish stands.
#[derive(Debug, Clone)]
pub struct Cancel {
    handle: Handle,
    scope: u32,
}

impl Cancel {
    pub(crate) fn new(handle: Handle, scope: u32) -> Self {
        Cancel { handle, scope }
    }

    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope
    }

    /// Open the cancel channel. Returns once the request is written;
    /// the scope's finish, which is the daemon's answer to it, arrives
    /// on the stream or the pending answer this was held beside. A
    /// second cancel changes nothing, and one after the finish is sent
    /// to nobody.
    pub async fn cancel(&self) -> Result<(), SendError> {
        self.handle.send_channel_request(self.scope, &CANCEL).await.map(|_| ())
    }
}
