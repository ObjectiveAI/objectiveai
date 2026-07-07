//! Typed consumer of the cli daemon's `/agents` endpoint — the read side
//! matching `objectiveai-cli`'s `websockets::websocket_agents` server.
//!
//! On connect the daemon sends one [`AgentEvent::Snapshot`], then streams
//! [`AgentEvent::Activated`] / [`AgentEvent::Deactivated`] deltas.
//! [`WebSocketAgentsListener`] IS a `Stream<Item = Result<AgentEvent, Error>>`:
//! one connection = one flat event stream (no per-run id demux, unlike
//! [`super::super::websocket_listener`]). The stream ends when the daemon
//! socket closes (a transport error surfaces as one final `Err` first);
//! reconnection is the caller's loop, and the daemon's published address
//! changes across restarts.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{SinkExt, Stream, StreamExt};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::AgentEvent;
use crate::cli::command::command_executor::websocket::AuthEnvelope;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon agents websocket: {0}")]
    Connect(tungstenite::Error),
    /// The established connection failed mid-stream.
    #[error("daemon agents websocket: {0}")]
    Ws(tungstenite::Error),
}

/// Unconnected configuration — [`WebSocketAgentsListener::new`] +
/// [`WebSocketAgentsListenerBuilder::signature`] +
/// [`WebSocketAgentsListenerBuilder::connect`].
pub struct WebSocketAgentsListenerBuilder {
    /// Full connect URL of the daemon's agents route, e.g.
    /// `ws://127.0.0.1:49152/agents`.
    url: String,
    /// Optional auth signature, sent in the [`AuthEnvelope`] preamble
    /// right after connecting.
    signature: Option<String>,
}

impl WebSocketAgentsListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim in the
    /// [`AuthEnvelope`] preamble — the connection's first text frame, the
    /// same shape every daemon route expects. Without it the daemon must
    /// be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Upgrade, send the auth preamble, and start the pump. The returned
    /// [`WebSocketAgentsListener`] is the flat event stream.
    pub async fn connect(self) -> Result<WebSocketAgentsListener, Error> {
        let upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        let (mut ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;

        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
        let auth = serde_json::to_string(&AuthEnvelope {
            signature: self.signature,
        })
        .expect("AuthEnvelope serialization is infallible");
        ws.send(tungstenite::Message::Text(auth.into()))
            .await
            .map_err(Error::Ws)?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<AgentEvent, Error>>();
        tokio::spawn(pump(ws, tx));
        Ok(WebSocketAgentsListener { rx })
    }
}

/// The flat agent-event stream — see the module docs. Construct via
/// [`WebSocketAgentsListener::new`].
pub struct WebSocketAgentsListener {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<AgentEvent, Error>>,
}

impl WebSocketAgentsListener {
    /// Start building a listener for the daemon's `/agents` URL (the
    /// daemon's published base address + `/agents`).
    pub fn new(url: impl Into<String>) -> WebSocketAgentsListenerBuilder {
        WebSocketAgentsListenerBuilder {
            url: url.into(),
            signature: None,
        }
    }
}

impl Stream for WebSocketAgentsListener {
    type Item = Result<AgentEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Read frames, parse each Text frame as one [`AgentEvent`], forward into
/// the channel. Skips unparseable frames; forwards a transport error as
/// one final `Err` then ends. Runs until the connection closes.
async fn pump(
    mut ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    tx: tokio::sync::mpsc::UnboundedSender<Result<AgentEvent, Error>>,
) {
    loop {
        match ws.next().await {
            Some(Ok(tungstenite::Message::Text(text))) => {
                match serde_json::from_str::<AgentEvent>(&text) {
                    Ok(event) => {
                        if tx.send(Ok(event)).is_err() {
                            break;
                        }
                    }
                    // Skip a frame we can't parse rather than tearing down.
                    Err(_) => continue,
                }
            }
            // Control / non-text frames: tungstenite answers pings itself.
            Some(Ok(tungstenite::Message::Close(_))) | None => break,
            Some(Ok(_)) => continue,
            Some(Err(e)) => {
                let _ = tx.send(Err(Error::Ws(e)));
                break;
            }
        }
    }
}
