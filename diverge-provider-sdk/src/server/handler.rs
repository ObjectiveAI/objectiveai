//! The top of the tree: one connection, read to its end.

use futures_util::{SinkExt as _, StreamExt as _};

use super::scope_handler::ScopeHandler;
use crate::encode::{Encode, Writer};
use crate::endpoints::ClientRequest;
use crate::frame::client::ClientFrame;
use crate::frame::server::ServerFrame;
use crate::shared::error::Error;
use crate::websocket::WebSocket;

/// One connection, and everything that happens on it.
///
/// A [`WebSocket`] goes in and [`run`](Self::run) reads it until it
/// ends, handing each frame to whatever is below. It is the only thing
/// in this module that touches the socket directly; everything deeper
/// is handed what it needs.
///
/// Either kind of socket. A provider that was connected to and a
/// provider that connected out answer the same frames the same way, so
/// this does not ask which.
///
/// # A tree, and this is the root
///
/// A connection holds scopes, a scope holds channels, and each level
/// knows only about the one under it. This level knows about scopes.
/// It does not know what a laboratory is, and it never will.
///
/// # Most of it is not written
///
/// What exists is the shape and the one path that needs no decisions:
/// an unreadable request gets an error and its own scope, because
/// there is nothing to consult about a request nobody can name. Every
/// other path reaches an [`unimplemented`] rather than guessing, and
/// they are marked with what they are waiting for.
pub struct Handler {
    socket: WebSocket,
    /// The next scope to mint.
    ///
    /// A counter because scopes are the server's to choose and nothing
    /// requires them to mean anything. It never wraps in practice: a
    /// connection would have to open four billion scopes first.
    next_scope: u32,
}

impl Handler {
    /// Take a socket somebody else finished making.
    ///
    /// Incoming or outgoing — see [`WebSocket`] for why that is not
    /// this type's business, and [`server`](super) for why this crate
    /// neither serves HTTP nor connects.
    pub fn new(socket: WebSocket) -> Self {
        Handler {
            socket,
            next_scope: 0,
        }
    }

    /// Read frames until the connection ends.
    ///
    /// Returns when the peer closes, when the socket errors, or when
    /// the stream simply stops. A connection ending is not a failure
    /// here — every scope on it ends with it, and there is nobody left
    /// to tell.
    pub async fn run(mut self) {
        while let Some(received) = self.socket.next().await {
            let bytes = match received {
                Ok(bytes) => bytes,
                // The socket failed, which is a connection that is
                // over. Everything a peer could have said wrongly was
                // skipped or is still to be decoded; only the
                // transport reaches here.
                Err(_) => return,
            };
            let frame = match ClientFrame::decode(&bytes) {
                Ok(frame) => frame,
                // A frame whose envelope will not parse — a truncated
                // header, a type this layer does not define, a
                // credential that is not UTF-8. There is no scope to
                // answer in, because a scope is what a request opens.
                Err(_) => unimplemented!("an unreadable frame"),
            };
            match frame {
                // Waiting on the provider to say what a credential
                // means. Nothing here can decide it: an unbrokered
                // token is whatever the two ends agreed, and this
                // crate is not one of the ends.
                ClientFrame::Auth(_) => unimplemented!("auth"),
                // A request nobody can name still gets a scope and an
                // answer. See `invalid`.
                ClientFrame::Request(ClientRequest::Invalid(_)) => {
                    let scope = self.mint();
                    self.invalid(scope).await;
                }
                ClientFrame::Request(request) => {
                    let scope = self.mint();
                    ScopeHandler::new(scope).handle(request).await;
                }
                // These belong to a scope that is already open, and
                // there is no register of open scopes yet — that
                // arrives with the scope handler.
                ClientFrame::ChannelRequest { .. }
                | ClientFrame::ChannelResponseAck { .. }
                | ClientFrame::ChannelResponse { .. }
                | ClientFrame::ChannelResponseFinish { .. } => {
                    unimplemented!("a frame inside an open scope")
                }
            }
        }
    }

    /// Take the next scope.
    fn mint(&mut self) -> u32 {
        let scope = self.next_scope;
        self.next_scope = self.next_scope.wrapping_add(1);
        scope
    }

    /// Answer a request nobody could name.
    ///
    /// Three frames and the scope is over: the ack that mints it, an
    /// error, and the finish.
    ///
    /// # Why it gets a scope at all
    ///
    /// Because the alternative is silence, and silence is not an
    /// answer a client can act on — a stream ends at its finish frame,
    /// so a client that got nothing would wait forever for a scope it
    /// believes it opened. Minting one costs a number and lets the
    /// rest of the connection carry on.
    ///
    /// # Why the payload is a bare error
    ///
    /// Every other response is some endpoint's frame, and an
    /// unreadable request is no endpoint's. So there is no tag to put
    /// in front of it: the payload is a
    /// [`shared::error::Error`](crate::shared::error::Error) and
    /// nothing else, which is the one shape that means the same thing
    /// without belonging to anything.
    async fn invalid(&mut self, scope: u32) {
        let mut payload = Vec::new();
        // Not `unimplemented` — impossible. Serializing a null cannot
        // fail, and writing into a `Vec` cannot either.
        Error::default()
            .encode(&mut Writer::new(&mut payload))
            .expect("a null error always serializes");
        self.send(&ServerFrame::ResponseAck { scope }).await;
        self.send(&ServerFrame::Response {
            scope,
            payload: &payload,
        })
        .await;
        self.send(&ServerFrame::ResponseFinish { scope }).await;
    }

    /// Write one frame, and stop caring if it does not arrive.
    ///
    /// A send that fails is a connection that is gone, and the loop
    /// finds that out on its next read. Nothing is retried, because a
    /// frame this peer did not receive is a frame about a scope it no
    /// longer has.
    async fn send(&mut self, frame: &ServerFrame<'_>) {
        let mut bytes = Vec::new();
        frame
            .encode(&mut Writer::new(&mut bytes))
            .unwrap_or_else(|error| match error {});
        let _ = self.socket.send(bytes.into()).await;
    }
}
