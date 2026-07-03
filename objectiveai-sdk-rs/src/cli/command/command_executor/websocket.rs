use std::pin::Pin;

use futures::{SinkExt, Stream, StreamExt};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

use crate::cli::command::{AgentArguments, CommandExecutor, CommandRequest, CommandResponse};

/// Execute commands against a remote cli daemon over WebSocket —
/// connection per command on the daemon's `/execute` route.
///
/// The daemon runs each request IN-PROCESS (no child binary) with the
/// envelope's [`AgentArguments`] applied as a per-request config
/// override (`mcp_session_id` is ignored server-side; the daemon's
/// filesystem layout and secret are never overridable). This is what
/// lets a consumer — notably the viewer — run on a different machine
/// than the cli: the only local requirement is network reach to the
/// daemon's published `ws://` address.
///
/// Wire contract (one connection per `execute`):
/// - client sends ONE text message, the [`ExecuteEnvelope`], then only
///   reads;
/// - the daemon sends one text message per stream item — exactly the
///   cli's stdout JSONL line shapes (a `T` JSON, or a
///   [`crate::cli::Error`] `{"type":"error",...}` line) — then closes;
/// - dropping the stream drops the connection, which cancels the
///   in-process run on the daemon.
///
/// Auth mirrors the daemon's broadcast route: when the daemon has a
/// `DAEMON_SECRET`, upgrades must carry the pre-derived
/// `sha256=<hex(SHA256(secret))>` as `X-DAEMON-SIGNATURE` — set it
/// verbatim via [`Self::signature`].
pub struct WebSocketExecutor {
    /// Full connect URL of the daemon's execute route, e.g.
    /// `ws://127.0.0.1:49152/execute`.
    url: String,
    /// Optional auth header value, sent verbatim as
    /// `X-DAEMON-SIGNATURE` on every upgrade.
    signature: Option<String>,
}

impl WebSocketExecutor {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            signature: None,
        }
    }

    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim as
    /// `X-DAEMON-SIGNATURE` on every upgrade. Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}

/// The one client→daemon message on an `/execute` connection: the
/// typed request (as its serde JSON) plus the optional per-request
/// identity override. The daemon deserializes this exact shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.command_executor.ExecuteEnvelope")]
pub struct ExecuteEnvelope {
    /// Per-request identity override, applied onto the daemon's own
    /// config for this run only (same semantics as the other
    /// executors' per-call override; `mcp_session_id` is ignored).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_arguments: Option<AgentArguments>,
    /// The serde JSON of a `cli::command::Request`.
    pub request: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The signature value isn't a valid HTTP header value.
    #[error("daemon signature is not a valid header value")]
    Signature,
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon execute websocket: {0}")]
    Connect(tungstenite::Error),
    /// The established connection failed mid-stream.
    #[error("daemon execute websocket: {0}")]
    Ws(tungstenite::Error),
    /// Serializing the request or decoding a response message failed.
    #[error("decode daemon execute message: {0}")]
    Json(serde_json::Error),
    /// Structured error emitted by the daemon on the stream.
    #[error("{0}")]
    Cli(crate::cli::Error),
    /// `execute_one` was called but the stream produced no items.
    #[error("daemon execute stream produced no items")]
    Empty,
}

/// Per-message untagged decode. `Err` is listed first so serde tries it
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

impl CommandExecutor for WebSocketExecutor {
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
        let envelope = ExecuteEnvelope {
            agent_arguments: agent_arguments.cloned(),
            request: serde_json::to_value(&request).map_err(Error::Json)?,
        };
        let envelope = serde_json::to_string(&envelope).map_err(Error::Json)?;

        let mut upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        if let Some(signature) = &self.signature {
            let value = HeaderValue::from_str(signature).map_err(|_| Error::Signature)?;
            upgrade.headers_mut().insert("X-DAEMON-SIGNATURE", value);
        }
        let (mut ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;
        ws.send(tungstenite::Message::Text(envelope))
            .await
            .map_err(Error::Ws)?;

        // Carry the socket in the unfold state; dropping the stream
        // drops the connection, which cancels the daemon-side run.
        let stream = futures::stream::unfold(Some(ws), |ws| async move {
            let mut ws = ws?;
            loop {
                match ws.next().await {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        let item = match serde_json::from_str::<Line<T>>(&text) {
                            Ok(line) => line.into(),
                            Err(e) => Err(Error::Json(e)),
                        };
                        return Some((item, Some(ws)));
                    }
                    // Control / non-text frames: not part of the item
                    // stream (tungstenite answers pings internally).
                    Some(Ok(tungstenite::Message::Close(_))) | None => return None,
                    Some(Ok(_)) => continue,
                    // Yield the transport error, then end the stream —
                    // `None` state short-circuits the next poll.
                    Some(Err(e)) => return Some((Err(Error::Ws(e)), None)),
                }
            }
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
