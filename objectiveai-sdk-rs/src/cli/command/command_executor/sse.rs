use std::pin::Pin;

use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};

use crate::cli::command::{AgentArguments, CommandExecutor, CommandRequest, CommandResponse};

/// Execute commands against a remote cli daemon over plain HTTP —
/// one POST per command to the daemon's `/execute` route, with the
/// result streamed back as Server-Sent Events.
///
/// The daemon runs each request IN-PROCESS (no child binary) with the
/// [`AgentArguments`] headers applied as a per-request config
/// override (`mcp_session_id` has no header; the daemon's
/// filesystem layout and secret are never overridable). This is what
/// lets a consumer — notably the viewer — run on a different machine
/// than the cli: the only local requirement is network reach to the
/// daemon's published `http://` address.
///
/// Wire contract (one request per `execute`):
/// - client POSTs the `cli::command::Request` serde JSON as the raw
///   body — nothing wraps it — with the auth signature in the
///   `X-OBJECTIVEAI-SIGNATURE` header and the [`AgentArguments`]
///   identity in the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` /
///   `-AGENT-ID` / `-AGENT-FULL-ID` / `-AGENT-REMOTE` /
///   `-RESPONSE-ID` / `-RESPONSE-IDS` request headers — the same
///   names the api stamps on outbound calls, one header per `Some`
///   field (`mcp_session_id` has no header). A missing header
///   DELETES that config field for the run — the daemon never
///   inherits its own resident value;
/// - the daemon replies `text/event-stream`, one SSE `data:` event per
///   stream item — exactly the cli's stdout JSONL line shapes (a `T`
///   JSON, or a [`crate::cli::Error`] `{"type":"error",...}` line) —
///   then ends the response body;
/// - the response body ending IS the end-of-stream marker (the HTTP
///   equivalent of the old WebSocket `Close`); dropping the stream
///   aborts the request, which cancels the in-process run on the daemon.
///
/// Auth is the `X-OBJECTIVEAI-SIGNATURE` header (the same header the
/// daemon's SSE watcher routes use): when the daemon has a
/// `DAEMON_SECRET`, the header must carry the pre-derived
/// `sha256=<hex(SHA256(secret))>` — set it verbatim via
/// [`Self::signature`] — or the daemon answers `401 Unauthorized`.
pub struct SseCommandExecutor {
    /// Full URL of the daemon's execute route, e.g.
    /// `http://127.0.0.1:49152/execute`.
    url: String,
    /// Optional auth signature, sent as the `X-OBJECTIVEAI-SIGNATURE`
    /// header on every request.
    signature: Option<String>,
}

impl SseCommandExecutor {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            signature: None,
        }
    }

    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` header on every request. Without it the
    /// daemon must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}

/// The first client→daemon frame on the `/laboratory` host-channel
/// WebSocket (the daemon's ONE remaining WebSocket) — the auth preamble,
/// always sent, even to a secretless daemon (`{"signature": null}`). A
/// daemon holding a `DAEMON_SECRET` verifies the signature and closes
/// the connection on a missing/invalid one; a secretless daemon consumes
/// the envelope and ignores the value. (The HTTP routes — `/execute`,
/// `/listen`, and the SSE watchers — authenticate by the
/// `X-OBJECTIVEAI-SIGNATURE` header instead.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.command_executor.AuthEnvelope")]
pub struct AuthEnvelope {
    /// The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, or
    /// `null` when the client has none.
    pub signature: Option<String>,
}


#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Building/sending the request, or connecting, failed.
    #[error("connect daemon execute: {0}")]
    Connect(reqwest::Error),
    /// The daemon answered a non-success status (e.g. `401` on a
    /// missing/invalid signature).
    #[error("daemon execute http {0}: {1}")]
    Http(reqwest::StatusCode, String),
    /// The SSE response body failed mid-stream.
    #[error("daemon execute sse stream: {0}")]
    Sse(String),
    /// Serializing the request or decoding a response event failed.
    #[error("decode daemon execute message: {0}")]
    Json(serde_json::Error),
    /// Structured error emitted by the daemon on the stream.
    #[error("{0}")]
    Cli(crate::cli::Error),
    /// `execute_one` was called but the stream produced no items.
    #[error("daemon execute stream produced no items")]
    Empty,
}

/// A transport error's FULL cause chain — `Display` alone hides the
/// interesting part (reqwest's "error decoding response body" says
/// nothing without the hyper cause underneath, e.g. "connection reset"
/// vs "connection closed before message completed").
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Per-event untagged decode. `Err` is listed first so serde tries it
/// before `Ok` — `cli::Error`'s `type:"error"` constant short-circuits
/// every non-error wire shape, then `Ok(T)` is the fallthrough.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Line<T> {
    Err(crate::cli::Error),
    Ok(T),
}

impl<T> From<Line<T>> for Result<T, Error> {
    fn from(line: Line<T>) -> Self {
        match line {
            Line::Err(e) => Err(Error::Cli(e)),
            Line::Ok(t) => Ok(t),
        }
    }
}

impl CommandExecutor for SseCommandExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut req = reqwest::Client::new()
            .post(&self.url)
            .header("Accept", "text/event-stream")
            .json(&request);
        if let Some(signature) = &self.signature {
            req = req.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        if let Some(args) = agent_arguments {
            if let Some(v) = &args.agent_instance_hierarchy {
                req = req.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", v);
            }
            if let Some(v) = &args.agent_id {
                req = req.header("X-OBJECTIVEAI-AGENT-ID", v);
            }
            if let Some(v) = &args.agent_full_id {
                req = req.header("X-OBJECTIVEAI-AGENT-FULL-ID", v);
            }
            if let Some(v) = &args.agent_remote {
                req = req.header("X-OBJECTIVEAI-AGENT-REMOTE", v);
            }
            if let Some(v) = &args.response_id {
                req = req.header("X-OBJECTIVEAI-RESPONSE-ID", v);
            }
            if let Some(v) = &args.response_ids {
                req = req.header("X-OBJECTIVEAI-RESPONSE-IDS", v);
            }
        }
        let response = req.send().await.map_err(Error::Connect)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Http(status, body.trim().to_string()));
        }

        // The SSE body owns the connection; dropping the returned stream
        // drops the body, which aborts the request and cancels the
        // daemon-side run. `eventsource-stream` parses without
        // reconnecting (the command stream is finite — body end = done).
        let events = response.bytes_stream().eventsource();
        let stream = events.map(|event| match event {
            Ok(event) => match serde_json::from_str::<Line<T>>(&event.data) {
                Ok(line) => line.into(),
                Err(e) => Err(Error::Json(e)),
            },
            // `EventStreamError::Transport` does NOT expose its inner
            // reqwest error via `source()` — unwrap it by hand so the
            // hyper cause chain (reset vs premature close vs framing)
            // actually reaches the message.
            Err(eventsource_stream::EventStreamError::Transport(e)) => {
                Err(Error::Sse(format!("transport: {}", error_chain(&e))))
            }
            Err(e) => Err(Error::Sse(error_chain(&e))),
        });

        Ok(Box::pin(stream))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, agent_arguments).await?;
        stream.next().await.ok_or(Error::Empty)?
    }
}
