//! HTTP client implementation for ObjectiveAI API.

use crate::error;
use eventsource_stream::Event as MessageEvent;
use futures::{SinkExt, Stream, StreamExt};
use reqwest_eventsource::{Event, RequestBuilderExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite;

/// HTTP client for making requests to the ObjectiveAI API.
///
/// Handles authentication, request building, and response parsing for both
/// unary and streaming endpoints.
///
/// # Example
///
/// ```ignore
/// let client = HttpClient::new(
///     reqwest::Client::new(),
///     None, // Use default address
///     Some("your-api-key"),
///     None, // user_agent
///     None, // x_title
///     None, // http_referer
///     None, // x_github_authorization
///     None, // x_openrouter_authorization
///     None, // x_mcp_authorization
///     None, // agent_instance_hierarchy
///     None, // mcp_call_timeout_ms
/// );
/// ```
#[derive(Debug, Clone)]
pub struct HttpClient {
    /// The underlying reqwest HTTP client.
    pub http_client: reqwest::Client,
    /// Base URL for API requests. Defaults to `https://api.objectiveai.dev`.
    pub address: String,
    /// API key for authentication. Sent as `Bearer` token in `Authorization` header.
    pub authorization: Option<Arc<String>>,
    /// Value for the `User-Agent` header.
    pub user_agent: Option<String>,
    /// Value for the `X-Title` header.
    pub x_title: Option<String>,
    /// Value for both `Referer` and `HTTP-Referer` headers.
    pub http_referer: Option<String>,
    /// Value for the `X-GITHUB-AUTHORIZATION` header.
    pub x_github_authorization: Option<Arc<String>>,
    /// Value for the `X-OPENROUTER-AUTHORIZATION` header.
    pub x_openrouter_authorization: Option<Arc<String>>,
    /// Values for the `X-MCP-AUTHORIZATION` header (JSON-encoded).
    pub x_mcp_authorization:
        Option<Arc<std::collections::HashMap<String, String>>>,
    /// Value for the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` header.
    pub agent_instance_hierarchy: Option<Arc<String>>,
    /// Value for the `X-MCP-CALL-TIMEOUT` header, in integer
    /// milliseconds: the per-request budget the API applies to each MCP
    /// CALL its proxy makes on this request's behalf (HTTP and ws://
    /// upstreams alike; never connects, never laboratory transfers).
    /// Unset ⇒ no header ⇒ the API applies NO call timeout.
    pub mcp_call_timeout_ms: Option<u64>,
}

impl HttpClient {
    /// Creates a new HTTP client.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The reqwest client to use for requests
    /// * `address` - Base URL for API requests (defaults to `https://api.objectiveai.dev`)
    /// * `authorization` - API key for authentication
    /// * `user_agent` - Optional User-Agent header value
    /// * `x_title` - Optional X-Title header value
    /// * `http_referer` - Optional Referer header value
    /// * `x_github_authorization` - Optional X-GITHUB-AUTHORIZATION header value
    /// * `x_openrouter_authorization` - Optional X-OPENROUTER-AUTHORIZATION header value
    /// * `x_mcp_authorization` - Optional X-MCP-AUTHORIZATION header value (HashMap)
    /// * `agent_instance_hierarchy` - Optional X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY header value
    /// * `mcp_call_timeout_ms` - Optional X-MCP-CALL-TIMEOUT header value
    ///   (integer ms; no env fallback — config/option-only)
    pub fn new(
        http_client: reqwest::Client,
        address: Option<impl Into<String>>,
        authorization: Option<impl Into<String>>,
        user_agent: Option<impl Into<String>>,
        x_title: Option<impl Into<String>>,
        http_referer: Option<impl Into<String>>,
        x_github_authorization: Option<impl Into<String>>,
        x_openrouter_authorization: Option<impl Into<String>>,
        x_mcp_authorization: Option<std::collections::HashMap<String, String>>,
        agent_instance_hierarchy: Option<impl Into<String>>,
        mcp_call_timeout_ms: Option<u64>,
    ) -> Self {
        #[cfg(feature = "env")]
        let env = |name: &str| -> Option<String> { std::env::var(name).ok() };

        Self {
            http_client,
            address: match address {
                Some(base) => base.into(),
                #[cfg(feature = "env")]
                None => env("OBJECTIVEAI_ADDRESS").unwrap_or_else(|| {
                    "https://api.objectiveai.dev".to_string()
                }),
                #[cfg(not(feature = "env"))]
                None => "https://api.objectiveai.dev".to_string(),
            },
            authorization: authorization.map(|k| Arc::new(k.into())).or_else(
                || {
                    #[cfg(feature = "env")]
                    {
                        env("OBJECTIVEAI_AUTHORIZATION").map(Arc::new)
                    }
                    #[cfg(not(feature = "env"))]
                    {
                        None
                    }
                },
            ),
            user_agent: user_agent.map(Into::into).or_else(|| {
                #[cfg(feature = "env")]
                {
                    env("USER_AGENT")
                }
                #[cfg(not(feature = "env"))]
                {
                    None
                }
            }),
            x_title: x_title.map(Into::into).or_else(|| {
                #[cfg(feature = "env")]
                {
                    env("X_TITLE")
                }
                #[cfg(not(feature = "env"))]
                {
                    None
                }
            }),
            http_referer: http_referer.map(Into::into).or_else(|| {
                #[cfg(feature = "env")]
                {
                    env("HTTP_REFERER")
                }
                #[cfg(not(feature = "env"))]
                {
                    None
                }
            }),
            x_github_authorization: x_github_authorization
                .map(|v| Arc::new(v.into()))
                .or_else(|| {
                    #[cfg(feature = "env")]
                    {
                        env("GITHUB_AUTHORIZATION").map(Arc::new)
                    }
                    #[cfg(not(feature = "env"))]
                    {
                        None
                    }
                }),
            x_openrouter_authorization: x_openrouter_authorization
                .map(|v| Arc::new(v.into()))
                .or_else(|| {
                    #[cfg(feature = "env")]
                    {
                        env("OPENROUTER_AUTHORIZATION").map(Arc::new)
                    }
                    #[cfg(not(feature = "env"))]
                    {
                        None
                    }
                }),
            x_mcp_authorization: x_mcp_authorization.map(Arc::new).or_else(
                || {
                    #[cfg(feature = "env")]
                    {
                        env("MCP_AUTHORIZATION")
                            .and_then(|v| serde_json::from_str(&v).ok())
                            .map(Arc::new)
                    }
                    #[cfg(not(feature = "env"))]
                    {
                        None
                    }
                },
            ),
            agent_instance_hierarchy: agent_instance_hierarchy.map(|v| Arc::new(v.into())).or_else(|| {
                #[cfg(feature = "env")]
                {
                    env("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY").map(Arc::new)
                }
                #[cfg(not(feature = "env"))]
                {
                    None
                }
            }),
            // Deliberately no env fallback: the caller (e.g. the daemon,
            // from its `api.mcp_call_timeout_ms` config) supplies it or not.
            mcp_call_timeout_ms,
        }
    }

    /// Builds a request with authentication and custom headers.
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<impl serde::Serialize>,
    ) -> reqwest::RequestBuilder {
        let url = format!(
            "{}/{}",
            self.address.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut request = self.http_client.request(method, &url);
        if let Some(authorization) = &self.authorization {
            let key = authorization
                .strip_prefix("Bearer ")
                .unwrap_or(authorization);
            request =
                request.header("authorization", format!("Bearer {}", key));
        }
        if let Some(user_agent) = &self.user_agent {
            request = request.header("user-agent", user_agent);
        }
        if let Some(x_title) = &self.x_title {
            request = request.header("x-title", x_title);
        }
        if let Some(http_referer) = &self.http_referer {
            request = request.header("referer", http_referer);
            request = request.header("http-referer", http_referer);
        }
        if let Some(token) = &self.x_github_authorization {
            request = request.header("X-GITHUB-AUTHORIZATION", token.as_str());
        }
        if let Some(token) = &self.x_openrouter_authorization {
            request =
                request.header("X-OPENROUTER-AUTHORIZATION", token.as_str());
        }
        if let Some(headers) = &self.x_mcp_authorization {
            if let Ok(json) = serde_json::to_string(headers.as_ref()) {
                request = request.header("X-MCP-AUTHORIZATION", json);
            }
        }
        if let Some(id) = &self.agent_instance_hierarchy {
            request = request.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", id.as_str());
        }
        if let Some(ms) = self.mcp_call_timeout_ms {
            request = request.header("X-MCP-CALL-TIMEOUT", ms.to_string());
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        request
    }

    /// Sends a unary (request-response) API call and deserializes the response.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected response type to deserialize into
    ///
    /// # Errors
    ///
    /// Returns [`super::HttpError`] if the request fails, returns a non-success status,
    /// or the response cannot be deserialized.
    pub async fn send_unary<T: serde::de::DeserializeOwned + Send + 'static>(
        &self,
        method: reqwest::Method,
        path: impl AsRef<str>,
        body: Option<impl serde::Serialize>,
    ) -> Result<T, super::HttpError> {
        let response = self
            .http_client
            .execute(
                self.request(method, path.as_ref(), body)
                    .build()
                    .map_err(super::HttpError::RequestError)?,
            )
            .await
            .map_err(super::HttpError::HttpError)?;
        let code = response.status();
        if code.is_success() {
            let text =
                response.text().await.map_err(super::HttpError::HttpError)?;
            let mut de = serde_json::Deserializer::from_str(&text);
            match serde_path_to_error::deserialize::<_, T>(&mut de) {
                Ok(value) => Ok(value),
                Err(e) => Err(super::HttpError::DeserializationError(e)),
            }
        } else {
            match response.text().await {
                Ok(text) => Err(super::HttpError::BadStatus {
                    code,
                    body: match serde_json::from_str::<serde_json::Value>(&text)
                    {
                        Ok(body) => body,
                        Err(_) => serde_json::Value::String(text),
                    },
                }),
                Err(_) => Err(super::HttpError::BadStatus {
                    code,
                    body: serde_json::Value::Null,
                }),
            }
        }
    }

    /// Sends a unary API call that expects no response body.
    ///
    /// Useful for DELETE or other operations that only return a status code.
    ///
    /// # Errors
    ///
    /// Returns [`super::HttpError`] if the request fails or returns a non-success status.
    pub async fn send_unary_no_response(
        &self,
        method: reqwest::Method,
        path: impl AsRef<str>,
        body: Option<impl serde::Serialize>,
    ) -> Result<(), super::HttpError> {
        let response = self
            .http_client
            .execute(
                self.request(method, path.as_ref(), body)
                    .build()
                    .map_err(super::HttpError::RequestError)?,
            )
            .await
            .map_err(super::HttpError::HttpError)?;
        let code = response.status();
        if code.is_success() {
            Ok(())
        } else {
            match response.text().await {
                Ok(text) => Err(super::HttpError::BadStatus {
                    code,
                    body: match serde_json::from_str::<serde_json::Value>(&text)
                    {
                        Ok(body) => body,
                        Err(_) => serde_json::Value::String(text),
                    },
                }),
                Err(_) => Err(super::HttpError::BadStatus {
                    code,
                    body: serde_json::Value::Null,
                }),
            }
        }
    }

    /// Sends a streaming API call using Server-Sent Events (SSE).
    ///
    /// Returns a stream of deserialized chunks. The stream automatically handles:
    /// - SSE `[DONE]` messages (filtered out)
    /// - Comment lines starting with `:` (filtered out)
    /// - Empty data lines (filtered out)
    /// - API errors embedded in stream data
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected chunk type to deserialize each SSE message into
    ///
    /// # Errors
    ///
    /// Returns [`super::HttpError`] if the stream cannot be established.
    pub async fn send_streaming<
        T: serde::de::DeserializeOwned + Send + 'static,
        P: AsRef<str> + Send,
        B: serde::Serialize + Send,
    >(
        &self,
        method: reqwest::Method,
        path: P,
        body: Option<B>,
    ) -> Result<
        impl Stream<Item = Result<T, super::HttpError>>
        + Send
        + 'static
        + use<T, P, B>,
        super::HttpError,
    > {
        // Stop the stream at [DONE] to prevent reqwest_eventsource from
        // auto-reconnecting. Uses take_while on the raw SSE events, then
        // maps/filters the remaining events into typed chunks.
        // Stamps X-Transport: sse so the API's transport dispatcher
        // routes this to the SSE branch (the API default is WS).
        Ok(
            self.request(method, path.as_ref(), body)
                .header("X-Transport", "sse")
                .eventsource()?
                .take_while(|result| {
                    let dominated = matches!(
                        result,
                        Ok(Event::Message(MessageEvent { data, .. })) if data == "[DONE]"
                    );
                    async move { !dominated }
                })
                .then(|result| async {
                    match result {
                        Ok(Event::Open) => None,
                        Ok(Event::Message(MessageEvent { data, .. }))
                            if data.starts_with(":")
                                || data.is_empty() =>
                        {
                            None
                        }
                        Ok(Event::Message(MessageEvent { data, .. })) => {
                            let mut de =
                                serde_json::Deserializer::from_str(&data);
                            Some(
                                match serde_path_to_error::deserialize::<_, T>(
                                    &mut de,
                                ) {
                                    Ok(value) => Ok(value),
                                    Err(e) => match serde_json::from_str::<error::ResponseError>(&data) {
                                        Ok(err) => Err(super::HttpError::ApiError(err)),
                                        Err(_) => Err(super::HttpError::DeserializationError(e)),
                                    },
                                }
                            )
                        }
                        Err(reqwest_eventsource::Error::InvalidStatusCode(
                            code,
                            response,
                        )) => match response.text().await {
                            Ok(body) => {
                                Some(Err(super::HttpError::BadStatus {
                                    code,
                                    body: match serde_json::from_str::<
                                        serde_json::Value,
                                    >(
                                        &body
                                    ) {
                                        Ok(body) => body,
                                        Err(_) => {
                                            serde_json::Value::String(body)
                                        }
                                    },
                                }))
                            }
                            Err(_) => Some(Err(super::HttpError::BadStatus {
                                code,
                                body: serde_json::Value::Null,
                            })),
                        },
                        Err(e) => Some(Err(super::HttpError::StreamError(e))),
                    }
                })
                .filter_map(|x| async { x }),
        )
    }

    /// WebSocket variant of [`Self::send_streaming`]. Opens a WS to
    /// the configured `address`, sends `body` as the first text
    /// frame, then demultiplexes inbound frames into:
    ///
    /// - Chunk frames (yielded on the returned [`Stream`]).
    /// - [`client_response::Response`](crate::client_objectiveai_mcp::client_response::Response)
    ///   frames (routed to the [`super::Notifier`]'s pending-id map).
    /// - [`server_request::Request`](crate::client_objectiveai_mcp::server_request::Request)
    ///   frames (dispatched to `handler`; the result is written back
    ///   as a `server_response::Response` echoing the request id).
    ///
    /// Both the returned `Stream` and the returned [`super::Notifier`]
    /// share the underlying WebSocket: dropping both stops the demux
    /// task and closes the connection cleanly. Dropping only one
    /// keeps the WS alive — useful when a caller wants to send
    /// notifies after the chunk stream has finished, or vice-versa.
    #[cfg(feature = "mcp")]
    pub async fn send_streaming_ws<Chunk, B, H, P>(
        &self,
        method: reqwest::Method,
        path: P,
        body: B,
        handler: H,
    ) -> Result<
        (
            impl Stream<Item = Result<Chunk, super::HttpError>>
            + Send
            + Unpin
            + 'static
            + use<Chunk, B, H, P>,
            super::Notifier,
        ),
        super::HttpError,
    >
    where
        Chunk: serde::de::DeserializeOwned + Send + 'static,
        B: serde::Serialize + Send + 'static,
        H: super::McpHandler,
        P: AsRef<str>,
    {
        use crate::client_objectiveai_mcp::{
            client_response::Response as ClientResponse,
            server_request::Request as ServerRequest,
        };
        use futures::stream::SplitStream;
        use tokio::net::TcpStream;
        use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

        // Translate the configured `address` (http(s)://...) into a
        // ws(s):// URL. Path is appended directly.
        let url = format!(
            "{}/{}",
            self.address.trim_end_matches('/'),
            path.as_ref().trim_start_matches('/')
        );
        let ws_url = if let Some(rest) = url.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = url.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            url.clone()
        };
        let _ = method; // axum's WS route is `any(...)`, method is ignored on the wire.

        // Build the upgrade request manually so we can apply the
        // same auth + custom headers `request()` does for HTTP.
        let mut req = tungstenite::handshake::client::Request::builder()
            .method("GET")
            .uri(&ws_url)
            .header(
                "Host",
                reqwest::Url::parse(&url)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_owned))
                    .unwrap_or_default(),
            )
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header(
                "Sec-WebSocket-Key",
                tungstenite::handshake::client::generate_key(),
            )
            .header("Sec-WebSocket-Version", "13")
            .header("X-Transport", "ws");
        if let Some(authorization) = &self.authorization {
            let key = authorization
                .strip_prefix("Bearer ")
                .unwrap_or(authorization.as_str());
            req = req.header("authorization", format!("Bearer {}", key));
        }
        if let Some(ua) = &self.user_agent {
            req = req.header("user-agent", ua);
        }
        if let Some(x_title) = &self.x_title {
            req = req.header("x-title", x_title);
        }
        if let Some(http_referer) = &self.http_referer {
            req = req.header("referer", http_referer);
            req = req.header("http-referer", http_referer);
        }
        if let Some(token) = &self.x_github_authorization {
            req = req.header("X-GITHUB-AUTHORIZATION", token.as_str());
        }
        if let Some(token) = &self.x_openrouter_authorization {
            req = req.header("X-OPENROUTER-AUTHORIZATION", token.as_str());
        }
        if let Some(headers) = &self.x_mcp_authorization {
            if let Ok(json) = serde_json::to_string(headers.as_ref()) {
                req = req.header("X-MCP-AUTHORIZATION", json);
            }
        }
        if let Some(id) = &self.agent_instance_hierarchy {
            req = req.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", id.as_str());
        }
        if let Some(ms) = self.mcp_call_timeout_ms {
            req = req.header("X-MCP-CALL-TIMEOUT", ms.to_string());
        }
        let req = req.body(()).map_err(|e| {
            super::HttpError::WsConnect(tungstenite::Error::Http(
                tungstenite::http::Response::builder()
                    .status(400)
                    .body(Some(e.to_string().into_bytes()))
                    .unwrap(),
            ))
        })?;

        let (ws_stream, _resp) = tokio_tungstenite::connect_async(req).await?;
        // TCP keepalive on the long-lived streaming WS: a silently-
        // dead API host would otherwise leave this stream parked
        // forever (an idle TCP connection has no liveness signal).
        match ws_stream.get_ref() {
            MaybeTlsStream::Plain(tcp) => crate::net::set_tcp_keepalive(tcp),
            MaybeTlsStream::Rustls(tls) => {
                crate::net::set_tcp_keepalive(tls.get_ref().0)
            }
            _ => {}
        }
        let (mut sink, rx_stream): (
            _,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ) = ws_stream.split();

        // Send the body as the first text frame.
        let body_frame = serde_json::to_string(&body)
            .map_err(super::HttpError::NotifySerialize)?;
        sink.send(tungstenite::Message::Text(body_frame.into()))
            .await
            .map_err(super::HttpError::NotifySend)?;

        // Build the per-connection state shared with Notifier + demux.
        let sink: super::notifier::SharedSink =
            Arc::new(tokio::sync::Mutex::new(sink));
        let pending: super::notifier::PendingNotifies =
            Arc::new(dashmap::DashMap::new());

        // mpsc the demux task pushes chunks (or terminal errors) into.
        // Use futures::channel::mpsc so the rx side is `impl Stream`
        // without pulling in tokio-stream.
        let (chunk_tx, chunk_rx) = futures::channel::mpsc::unbounded::<
            Result<Chunk, super::HttpError>,
        >();

        let demux_sink = sink.clone();
        let demux_pending = pending.clone();
        let handler = Arc::new(handler);
        tokio::spawn(async move {
            let mut rx_stream = rx_stream;
            let mut chunk_tx = chunk_tx;
            loop {
                let msg = match rx_stream.next().await {
                    Some(m) => m,
                    None => break,
                };
                let text = match msg {
                    Ok(tungstenite::Message::Text(t)) => {
                        let s = t.to_string();
                        s
                    }
                    Ok(tungstenite::Message::Binary(_)) => {
                        continue;
                    }
                    Ok(
                        tungstenite::Message::Ping(_)
                        | tungstenite::Message::Pong(_),
                    ) => continue,
                    Ok(tungstenite::Message::Close(_)) => {
                        break;
                    }
                    Ok(tungstenite::Message::Frame(_)) => continue,
                    Err(_) => {
                        break;
                    }
                };

                // Classification: try client_response, then
                // server_request, then chunk. Order matters — chunks
                // tend to have many fields; the envelopes have a
                // distinctive `id` + tagged `type`.
                if let Ok(response) =
                    serde_json::from_str::<ClientResponse>(&text)
                {
                    let id = response.id().to_string();
                    if let Some((_, tx)) = demux_pending.remove(&id) {
                        let _ = tx.send(response);
                    }
                    continue;
                }
                if let Ok(request) =
                    serde_json::from_str::<ServerRequest>(&text)
                {
                    let id = request.id.clone();
                    let handler = handler.clone();
                    let demux_sink = demux_sink.clone();
                    tokio::spawn(async move {
                        let id = id;
                        // Handler returns the full response (incl.
                        // matching id); we just frame + write it.
                        let response = handler.handle(request).await;
                        let frame = match serde_json::to_string(&response) {
                            Ok(s) => s,
                            Err(_) => {
                                return;
                            }
                        };
                        let mut guard = demux_sink.lock().await;
                        let send_result = guard
                            .send(tungstenite::Message::Text(frame.into()))
                            .await;
                    });
                    continue;
                }

                // Chunk path.
                let mut de = serde_json::Deserializer::from_str(&text);
                match serde_path_to_error::deserialize::<_, Chunk>(&mut de) {
                    Ok(chunk) => {
                        if chunk_tx.unbounded_send(Ok(chunk)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        // Try to parse as a ResponseError before
                        // surfacing the deserialization failure.
                        let err = match serde_json::from_str::<
                            error::ResponseError,
                        >(&text)
                        {
                            Ok(api_err) => super::HttpError::ApiError(api_err),
                            Err(_) => super::HttpError::DeserializationError(e),
                        };
                        let _ = chunk_tx.unbounded_send(Err(err));
                        break;
                    }
                }
            }
            // Make sure any awaiting Notifier futures unblock when
            // we exit — dropping the dashmap fires every oneshot
            // Sender's drop, which causes the rx side to error.
            drop(demux_pending);
            drop(chunk_tx);
        });

        let notifier = super::Notifier::new(sink, pending);
        Ok((chunk_rx, notifier))
    }
}
