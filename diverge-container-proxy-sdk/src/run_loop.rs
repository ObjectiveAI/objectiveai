//! The loop: the request received, the chunks sent back.
//!
//! An agent container's harness is a program with one job — run the
//! loop the server asks for — and this is how it is asked. The
//! harness attaches to the proxy at `/run-loop/agent` and waits; the
//! server hands the proxy the request at `/run-loop`; the proxy hands
//! it on. Everything the harness sends back is relayed to the server
//! verbatim: one message per chunk, then the close.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the loop has said a single thing is a
//! real error — [`RunLoopHandle::error`], the wire's `Error` frame,
//! then the close: the agent value the image will not take, a key the
//! vault does not hold, a history that will not open, the first
//! fetch. A failure AFTER output is not: it is a fatal notification
//! chunk, sent with [`RunLoopHandle::send`] like any other, then
//! [`RunLoopHandle::finish`] — part of the loop's output, so a caller
//! can tell a partial result from a whole one. One rule, and the
//! first item decides which side of it a failure is on.

use diverge_provider_sdk::container_proxy::run_loop;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::shared::containers::run_loop::response;
use diverge_provider_sdk::shared::error::Error as WireError;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{Client, Error};

impl Client {
    /// Attach, and wait for the request.
    ///
    /// Returns when the server has handed the proxy a request, however
    /// long that takes — nothing times anything out — with the request
    /// and the handle the loop's chunks go out on. A second attachment
    /// while one is held is the proxy's `409`,
    /// [`Error::RunLoopStatus`].
    pub async fn run_loop(&self) -> Result<(run_loop::request::Request, RunLoopHandle), Error> {
        let (mut socket, _) =
            tokio_tungstenite::connect_async(crate::ws_url("/run-loop/agent"))
                .await
                .map_err(|error| match error {
                    tungstenite::Error::Http(response) => {
                        Error::RunLoopStatus(response.status().as_u16())
                    }
                    error => Error::RunLoopConnect(error),
                })?;
        let request = loop {
            match socket.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    break run_loop::request::Request::decode(&bytes)
                        .map_err(Error::RunLoopRequest)?;
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Text(_) | Message::Close(_))) | None => {
                    return Err(Error::RunLoopClosed);
                }
                Some(Err(error)) => return Err(Error::RunLoopSocket(error)),
            }
        };
        Ok((request, RunLoopHandle { socket }))
    }
}

/// The loop's socket, open: what the loop says goes out here.
///
/// Dropping this without [`finish`](Self::finish) or
/// [`error`](Self::error) is the loop dying, and the server is told
/// so by the proxy. A harness that is done says which.
#[must_use = "dropping the handle unfinished is the loop dying"]
pub struct RunLoopHandle {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RunLoopHandle {
    /// One chunk of the loop.
    pub async fn send(&mut self, chunk: response::AgenticLoopChunk) -> Result<(), Error> {
        self.frame(response::Frame::Chunk(chunk)).await
    }

    /// The real error: there was no loop to report on, or nothing
    /// more to say. Sent, then the close.
    pub async fn error(mut self, error: WireError) -> Result<(), Error> {
        self.frame(response::Frame::Error(error)).await?;
        self.socket.close(None).await.map_err(Error::RunLoopSocket)
    }

    /// The loop is over: the clean close.
    pub async fn finish(mut self) -> Result<(), Error> {
        self.socket.close(None).await.map_err(Error::RunLoopSocket)
    }

    async fn frame(&mut self, frame: response::Frame) -> Result<(), Error> {
        let mut bytes = Vec::new();
        frame
            .encode(&mut Writer::new(&mut bytes))
            .map_err(Error::RunLoopEncode)?;
        self.socket
            .send(Message::Binary(bytes.into()))
            .await
            .map_err(Error::RunLoopSocket)
    }
}
