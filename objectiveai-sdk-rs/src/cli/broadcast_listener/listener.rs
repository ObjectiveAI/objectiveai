//! Typed consumer of the cli daemon's `/listen` broadcast SSE — the
//! read side matching [`crate::cli::command::SseCommandExecutor`]'s
//! write side.
//!
//! The daemon broadcasts every CLI run as one request frame
//! (`{…context, id, value: <leaf Request>}`), then that run's response
//! frames (bare `{id, value}` wrappers — no type tag; the opening
//! request already told us how to deserialize the id's items), then
//! exactly one terminator (`{id, end: true}`, the SDK
//! [`super::ListenerEnd`]). The `id` is the whole routing story: terminator
//! by `end: true`, response when the id is already open, request
//! otherwise.
//!
//! [`BroadcastListener`] IS a `Stream`: it yields one [`ListenerExecution`] envelope
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
//! Frames are parsed as a borrowed envelope whose `value` stays a
//! [`serde_json::value::RawValue`] — deserializing the actual body is
//! DEFERRED until the dispatch knows exactly what type it is, then it
//! parses straight from the wire text. No `serde_json::Value`
//! intermediate, so high-precision numbers (`Decimal` scores and the
//! like) never round-trip through `f64`.

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde_json::value::RawValue;

use crate::cli::command::AgentArguments;

use crate::cli::command::ListenerExecution;
use super::dispatch::{RunFeed, open_run};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE stream
    /// failed.
    #[error("connect daemon listen sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build.
    #[error("daemon listen sse http client: {0}")]
    Client(#[from] reqwest::Error),
    /// The SSE stream failed (or ended) mid-run — the caller reconnects.
    #[error("daemon listen sse stream: {0}")]
    Stream(String),
}

/// Unconnected configuration — [`BroadcastListener::new`] +
/// [`BroadcastListenerBuilder::signature`] +
/// [`BroadcastListenerBuilder::connect`].
pub struct BroadcastListenerBuilder {
    /// Full `http`/`https` URL of the daemon's listen SSE route, e.g.
    /// `http://127.0.0.1:49152/listen`.
    url: String,
    /// Optional auth signature, sent as the `X-OBJECTIVEAI-SIGNATURE`
    /// request header.
    signature: Option<String>,
}

impl BroadcastListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
    /// running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Open the SSE stream and start the pump. The returned
    /// [`BroadcastListener`] is the root envelope stream; connection/auth
    /// failures surface as the first `Err` item.
    pub async fn connect(self) -> Result<BroadcastListener, Error> {
        let source = connect_sse(&self.url, self.signature.as_deref())?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<ListenerExecution, Error>>();
        tokio::spawn(pump(source, tx));
        Ok(BroadcastListener { rx })
    }
}

/// The root run stream — see the module docs. Construct via
/// [`BroadcastListener::new`].
pub struct BroadcastListener {
    rx: tokio::sync::mpsc::UnboundedReceiver<Result<ListenerExecution, Error>>,
}

impl BroadcastListener {
    /// Start building a listener for the daemon's `/listen` URL (the
    /// daemon's published base address + `/listen`).
    pub fn new(url: impl Into<String>) -> BroadcastListenerBuilder {
        BroadcastListenerBuilder {
            url: url.into(),
            signature: None,
        }
    }
}

impl Stream for BroadcastListener {
    type Item = Result<ListenerExecution, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Borrowed view of one broadcast frame. `value` stays a
/// [`RawValue`] — the actual body is deserialized later, straight
/// from this text, once the dispatch knows exactly what type it is.
/// The remaining fields are the cheap discriminators plus the
/// producer's context. A borrowed superset of the wire vocabulary
/// (`ListenerRequest`/`ListenerResponse`/`ListenerEnd` in
/// `super::wire`) for zero-copy dispatch.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(serde::Deserialize)]
struct Frame<'a> {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    end: Option<bool>,
    #[serde(default, borrow)]
    value: Option<&'a RawValue>,
    #[serde(default)]
    agent_instance_hierarchy: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    agent_full_id: Option<String>,
    #[serde(default)]
    agent_remote: Option<String>,
    #[serde(default)]
    response_id: Option<String>,
    #[serde(default)]
    response_ids: Option<String>,
    #[serde(default)]
    plugin_owner: Option<String>,
    #[serde(default)]
    plugin_repository: Option<String>,
    #[serde(default)]
    plugin_version: Option<String>,
    /// Task-scheduler-fired run marker (typed BOOL on the wire; absent
    /// = false on frames from older daemons).
    #[serde(default)]
    task: bool,
}

impl Frame<'_> {
    /// The producer's identity off the request frame's context fields
    /// — the plugin caller trio included: broadcast frames are
    /// DAEMON-authored, so the trio here is trustworthy (unlike
    /// inbound wire requests, where the daemon ignores any claim).
    fn agent_arguments(&mut self) -> AgentArguments {
        AgentArguments {
            agent_instance_hierarchy: self.agent_instance_hierarchy.take(),
            agent_id: self.agent_id.take(),
            agent_full_id: self.agent_full_id.take(),
            agent_remote: self.agent_remote.take(),
            response_id: self.response_id.take(),
            response_ids: self.response_ids.take(),
            plugin_owner: self.plugin_owner.take(),
            plugin_repository: self.plugin_repository.take(),
            plugin_version: self.plugin_version.take(),
            task: self.task,
        }
    }
}

/// The distribution task: read broadcast frames, open a [`ListenerExecution`] per
/// request frame, feed its response frames, close it on the
/// terminator. Runs until the connection ends — deliberately not tied
/// to the root stream's lifetime (see module docs).
async fn pump(
    mut source: reqwest_eventsource::EventSource,
    tx: tokio::sync::mpsc::UnboundedSender<Result<ListenerExecution, Error>>,
) {
    let mut feeds: HashMap<String, RunFeed> = HashMap::new();
    // Ids whose run was skipped (unrecognized / undeserializable
    // request): remembered so their tag-less response frames aren't
    // re-probed as requests.
    let mut skipped: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        match source.next().await {
            Some(Ok(Event::Open)) => continue,
            Some(Ok(Event::Message(message))) => {
                let text = message.data;
                let Ok(mut frame) = serde_json::from_str::<Frame>(&text) else {
                    continue;
                };
                let Some(id) = frame.id.take() else {
                    continue;
                };
                // Frames carry no type tag: the id is the whole routing
                // story. Terminator by its `end: true` marker; response
                // when the id is already open (or skipped); request
                // otherwise.
                if frame.end == Some(true) {
                    // Terminator: exactly one per id — the run is done.
                    if let Some(feed) = feeds.remove(&id) {
                        feed.close();
                    }
                    skipped.remove(&id);
                } else if let Some(feed) = feeds.get_mut(&id) {
                    // Response frame for a known run: hand the raw body
                    // to the feed, which deserializes it as its exact
                    // item type (known from the opening request).
                    if let Some(value) = frame.value {
                        feed.push(value);
                    }
                } else if skipped.contains(&id) {
                    // Response frame for a run we chose not to open.
                } else {
                    // Request frame: open the run and yield its envelope.
                    // An unrecognized `path_type` (or a request these
                    // types predate) opens nothing — the run is skipped
                    // and its later frames drop above.
                    let Some(request) = frame.value else {
                        continue;
                    };
                    let agent_arguments = frame.agent_arguments();
                    let Some((run, feed)) = open_run(request, agent_arguments) else {
                        skipped.insert(id);
                        continue;
                    };
                    feeds.insert(id, feed);
                    // Root receiver gone: keep pumping for the nested
                    // streams already handed out.
                    let _ = tx.send(Ok(run));
                }
            }
            // Stream ended or errored — end the pump (the caller
            // reconnects); a final `Err` surfaces the FULL cause chain
            // first (`Display` alone hides the transport cause).
            Some(Err(e)) => {
                let mut message = e.to_string();
                let mut source = std::error::Error::source(&e);
                while let Some(cause) = source {
                    message.push_str(": ");
                    message.push_str(&cause.to_string());
                    source = cause.source();
                }
                let _ = tx.send(Err(Error::Stream(message)));
                break;
            }
            None => break,
        }
    }
    // Connection over: close every still-open run (unresolved unary
    // futures settle with the synthesized "run ended" error; streams
    // end).
    for (_, feed) in feeds.drain() {
        feed.close();
    }
}

/// Open the daemon's `/listen` SSE stream: request `text/event-stream`
/// and stamp `X-OBJECTIVEAI-SIGNATURE` when a signature is present. `url`
/// MUST be an `http`/`https` URL.
fn connect_sse(
    url: &str,
    signature: Option<&str>,
) -> Result<reqwest_eventsource::EventSource, Error> {
    let client = reqwest::Client::builder().build()?;
    let mut request = client.get(url).header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
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

/// Error-first per-frame decode, straight from the raw wire text (no
/// `serde_json::Value` intermediate — deferred until the exact target
/// type is known, so precision survives). The same order as the
/// executors' `Line<T>`: `cli::Error`'s `type:"error"` constant
/// short-circuits every non-error wire shape, then `T` is the
/// fallthrough, and a value that is neither becomes a synthesized
/// error carrying the raw text.
pub(crate) fn decode_item<T: serde::de::DeserializeOwned>(
    value: &RawValue,
) -> Result<T, crate::cli::Error> {
    if let Ok(error) = serde_json::from_str::<crate::cli::Error>(value.get()) {
        return Err(error);
    }
    match serde_json::from_str::<T>(value.get()) {
        Ok(item) => Ok(item),
        Err(_) => Err(crate::cli::Error {
            r#type: crate::cli::ErrorType::Error,
            level: Some(crate::cli::Level::Error),
            fatal: None,
            message: serde_json::from_str::<serde_json::Value>(value.get())
                .unwrap_or_else(|_| serde_json::Value::String(value.get().to_string())),
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
