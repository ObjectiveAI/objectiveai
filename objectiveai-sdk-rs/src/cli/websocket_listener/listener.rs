//! Typed consumer of the cli daemon's `/listen` broadcast — the read
//! side matching [`crate::cli::command::WebSocketExecutor`]'s write
//! side.
//!
//! The daemon broadcasts every CLI run as one request frame
//! (`{…context, id, value: <leaf Request>}` — no top-level
//! `path_type`), then that run's response frames
//! (`{id, path_type, value: <item>}`), then exactly one terminator
//! (`{id, path_type, end: true}`, the SDK `RootViewerEnd`).
//!
//! [`WebSocketListener`] IS a `Stream`: it yields one [`Run`] envelope
//! per announced run, discriminated over the run's REQUEST — each
//! variant carries the actual leaf request, the producer's
//! [`AgentArguments`], and the response as either a
//! [`UnaryResponse`] future (unary leaves) or a
//! [`ResponseItemStream`] (streaming leaves). The root stream never
//! yields response items; the nested future/stream inside each
//! envelope does. Any nested future/stream can yield in-band
//! [`crate::cli::Error`]s — error lines ride every run.
//!
//! One listener = one connection: the root stream ends when the
//! daemon's socket closes (a transport error surfaces as one final
//! `Err` item first); reconnection is the caller's loop, and the
//! daemon's published address changes across restarts. Dropping the
//! root stream does NOT stop the pump — envelopes already yielded
//! keep their nested streams flowing until the connection ends.
//!
//! Precision caveat: frames pass through a `serde_json::Value`
//! intermediate before the typed deserialization, and without
//! serde_json's `arbitrary_precision` feature that routes numbers
//! through `f64` — high-precision `Decimal` fields can lose digits.
//! Known follow-up: parse frames with a `&RawValue` `value` field and
//! deserialize `T` straight from the raw text.

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

use crate::cli::command::AgentArguments;

use super::run::{RunFeed, open_run};
use super::Run;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The signature value isn't a valid HTTP header value.
    #[error("daemon signature is not a valid header value")]
    Signature,
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon listen websocket: {0}")]
    Connect(tungstenite::Error),
    /// The established connection failed mid-stream.
    #[error("daemon listen websocket: {0}")]
    Ws(tungstenite::Error),
}

/// Unconnected configuration — [`WebSocketListener::new`] +
/// [`WebSocketListenerBuilder::signature`] +
/// [`WebSocketListenerBuilder::connect`].
pub struct WebSocketListenerBuilder {
    /// Full connect URL of the daemon's listen route, e.g.
    /// `ws://127.0.0.1:49152/listen`.
    url: String,
    /// Optional auth header value, sent verbatim as
    /// `X-DAEMON-SIGNATURE` on the upgrade.
    signature: Option<String>,
}

impl WebSocketListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim as
    /// `X-DAEMON-SIGNATURE`. Without it the daemon must be running
    /// without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Upgrade and start the pump. The returned [`WebSocketListener`]
    /// is the root envelope stream.
    pub async fn connect(self) -> Result<WebSocketListener, Error> {
        let mut upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        if let Some(signature) = &self.signature {
            let value = HeaderValue::from_str(signature).map_err(|_| Error::Signature)?;
            upgrade.headers_mut().insert("X-DAEMON-SIGNATURE", value);
        }
        let (ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Run, Error>>();
        tokio::spawn(pump(ws, tx));
        Ok(WebSocketListener { rx })
    }
}

/// The root run stream — see the module docs. Construct via
/// [`WebSocketListener::new`].
pub struct WebSocketListener {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<Run, Error>>,
}

impl WebSocketListener {
    /// Start building a listener for the daemon's `/listen` URL (the
    /// daemon's published base address + `/listen`).
    pub fn new(url: impl Into<String>) -> WebSocketListenerBuilder {
        WebSocketListenerBuilder {
            url: url.into(),
            signature: None,
        }
    }
}

impl Stream for WebSocketListener {
    type Item = Result<Run, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// The distribution task: read broadcast frames, open a [`Run`] per
/// request frame, feed its response frames, close it on the
/// terminator. Runs until the connection ends — deliberately not tied
/// to the root stream's lifetime (see module docs).
async fn pump(
    mut ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    tx: tokio::sync::mpsc::UnboundedSender<Result<Run, Error>>,
) {
    let mut feeds: HashMap<String, RunFeed> = HashMap::new();
    loop {
        match ws.next().await {
            Some(Ok(tungstenite::Message::Text(text))) => {
                let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let Some(id) = frame.get("id").and_then(|i| i.as_str()) else {
                    continue;
                };
                if frame.get("path_type").is_none() {
                    // Request frame: open the run and yield its envelope.
                    // An unrecognized `path_type` (or a request these
                    // types predate) opens nothing — the run is skipped
                    // and, with its id untracked, its frames drop below.
                    let Some(request) = frame.get("value") else {
                        continue;
                    };
                    let path_type = request
                        .get("path_type")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string();
                    let agent_arguments = extract_agent_arguments(&frame);
                    let Some((run, feed)) = open_run(&path_type, request.clone(), agent_arguments)
                    else {
                        continue;
                    };
                    feeds.insert(id.to_string(), feed);
                    // Root receiver gone: keep pumping for the nested
                    // streams already handed out.
                    let _ = tx.send(Ok(run));
                } else if frame.get("end").and_then(|e| e.as_bool()) == Some(true) {
                    // Terminator: exactly one per id — the run is done.
                    if let Some(feed) = feeds.remove(id) {
                        feed.close();
                    }
                } else if let Some(feed) = feeds.get_mut(id) {
                    // Response frame for a known run.
                    let value = frame
                        .get("value")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    feed.push(value);
                }
            }
            // Control / non-text frames: not part of the frame stream
            // (tungstenite answers pings internally).
            Some(Ok(tungstenite::Message::Close(_))) | None => break,
            Some(Ok(_)) => continue,
            Some(Err(e)) => {
                let _ = tx.send(Err(Error::Ws(e)));
                break;
            }
        }
    }
    // Connection over: close every still-open run (unresolved unary
    // futures settle with the synthesized "run ended" error; streams
    // end).
    for (_, feed) in feeds.drain() {
        feed.close();
    }
}

/// The producer's identity off a request frame's context fields.
/// `mcp_session_id` is never teed onto the broadcast, so it's always
/// `None`; the frame's `plugin_*` coordinates are not agent arguments
/// and are dropped.
fn extract_agent_arguments(frame: &serde_json::Value) -> AgentArguments {
    let field = |name: &str| {
        frame
            .get(name)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    AgentArguments {
        agent_instance_hierarchy: field("agent_instance_hierarchy"),
        agent_id: field("agent_id"),
        agent_full_id: field("agent_full_id"),
        agent_remote: field("agent_remote"),
        response_id: field("response_id"),
        response_ids: field("response_ids"),
        mcp_session_id: None,
    }
}

/// One run's unary response: resolves on the run's FIRST response
/// frame — `Ok(T)` for the typed response, `Err` for an in-band
/// [`crate::cli::Error`] line — or, when the run terminates without
/// any response frame, an `Err` with a synthesized error (mirroring
/// the execute functions' no-output error).
pub struct UnaryResponse<T> {
    rx: tokio::sync::oneshot::Receiver<Result<T, crate::cli::Error>>,
    path_type: String,
}

impl<T> UnaryResponse<T> {
    pub(crate) fn channel(
        path_type: String,
    ) -> (
        Self,
        tokio::sync::oneshot::Sender<Result<T, crate::cli::Error>>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self { rx, path_type }, tx)
    }
}

impl<T> std::future::Future for UnaryResponse<T> {
    type Output = Result<T, crate::cli::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            // Sender dropped without resolving (run ended with no
            // response frame, or the run's feed was torn down).
            Poll::Ready(Err(_)) => Poll::Ready(Err(ended_without_response(&self.path_type))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// One run's response-item stream: one item per response frame —
/// `Ok(T)` typed, `Err` for in-band [`crate::cli::Error`] lines (a
/// frame that decodes as neither is wrapped into a synthesized error
/// carrying the raw value; nothing is silently dropped). Ends on the
/// run's terminator (or the connection ending). Buffered: a slow
/// consumer never stalls the pump.
pub struct ResponseItemStream<T> {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<T, crate::cli::Error>>,
}

impl<T> ResponseItemStream<T> {
    pub(crate) fn channel() -> (
        Self,
        tokio::sync::mpsc::UnboundedSender<Result<T, crate::cli::Error>>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Self { rx }, tx)
    }
}

impl<T> Stream for ResponseItemStream<T> {
    type Item = Result<T, crate::cli::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Error-first per-frame decode — the same order as the executors'
/// `Line<T>`: `cli::Error`'s `type:"error"` constant short-circuits
/// every non-error wire shape, then `T` is the fallthrough, and a
/// value that is neither becomes a synthesized error carrying the raw
/// value.
pub(crate) fn decode_item<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, crate::cli::Error> {
    if let Ok(error) = serde_json::from_value::<crate::cli::Error>(value.clone()) {
        return Err(error);
    }
    match serde_json::from_value::<T>(value.clone()) {
        Ok(item) => Ok(item),
        Err(_) => Err(crate::cli::Error {
            r#type: crate::cli::ErrorType::Error,
            level: Some(crate::cli::Level::Error),
            fatal: None,
            message: value,
        }),
    }
}

/// The synthesized error a unary run resolves with when it terminates
/// before any response frame.
pub(crate) fn ended_without_response(path_type: &str) -> crate::cli::Error {
    crate::cli::Error {
        r#type: crate::cli::ErrorType::Error,
        level: Some(crate::cli::Level::Error),
        fatal: None,
        message: serde_json::Value::String(format!(
            "{path_type}: run ended before any response item"
        )),
    }
}
