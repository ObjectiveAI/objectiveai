//! MCP client for creating connections to MCP servers.

use std::time::Duration;

use indexmap::IndexMap;

/// Client for creating MCP connections.
///
/// Holds shared configuration (HTTP client, headers, backoff parameters)
/// and creates [`Connection`](super::Connection) instances via
/// [`connect`](Client::connect).
#[derive(Debug, Clone)]
pub struct Client {
    /// HTTP client for making requests.
    pub http_client: reqwest::Client,
    /// User-Agent header value.
    pub user_agent: String,
    /// X-Title header value.
    pub x_title: String,
    /// Referer header value.
    pub http_referer: String,
    /// Timeout for the initial connection (initialize request).
    /// `None` = no timeout — the caller waits as long as the upstream
    /// takes (e.g. the daemon, which never bounds its own MCP calls).
    pub connect_timeout: Option<Duration>,

    /// Current backoff interval for retry logic.
    pub backoff_current_interval: Duration,
    /// Initial backoff interval for retry logic.
    pub backoff_initial_interval: Duration,
    /// Randomization factor for backoff jitter.
    pub backoff_randomization_factor: f64,
    /// Multiplier for exponential backoff growth.
    pub backoff_multiplier: f64,
    /// Maximum backoff interval.
    pub backoff_max_interval: Duration,
    /// Maximum total time to spend on retries.
    pub backoff_max_elapsed_time: Duration,
    /// Timeout for individual RPC calls after connection is established.
    /// `None` = no timeout (see [`Client::connect_timeout`]).
    pub call_timeout: Option<Duration>,
}

/// Apply an optional per-request timeout to a builder: `None` leaves the
/// request unbounded (reqwest applies no default request timeout).
pub(crate) fn apply_timeout(
    request: reqwest::RequestBuilder,
    timeout: Option<Duration>,
) -> reqwest::RequestBuilder {
    match timeout {
        Some(timeout) => request.timeout(timeout),
        None => request,
    }
}

impl Client {
    /// Creates a new MCP client.
    pub fn new(
        http_client: reqwest::Client,
        user_agent: String,
        x_title: String,
        http_referer: String,
        connect_timeout: Option<Duration>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Option<Duration>,
    ) -> Self {
        Self {
            http_client,
            user_agent,
            x_title,
            http_referer,
            connect_timeout,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            call_timeout,
        }
    }

    /// Build the canonical header map that the client stamps on every
    /// request opened under this client / connection. The supplied
    /// caller map (if any) wins on conflict — defaults are inserted
    /// only when the caller didn't already provide them. The merged
    /// map is computed once at the top of `connect_once` and reused
    /// across all three handshake requests + handed to the resulting
    /// [`Connection`] for every later RPC.
    fn headers(
        &self,
        supplied: Option<IndexMap<String, String>>,
    ) -> IndexMap<String, String> {
        let mut out = supplied.unwrap_or_default();
        out.entry("User-Agent".to_string())
            .or_insert_with(|| self.user_agent.clone());
        out.entry("X-Title".to_string())
            .or_insert_with(|| self.x_title.clone());
        out.entry("Referer".to_string())
            .or_insert_with(|| self.http_referer.clone());
        out.entry("HTTP-Referer".to_string())
            .or_insert_with(|| self.http_referer.clone());
        out
    }

    /// Connects to an MCP server using the Streamable HTTP transport.
    ///
    /// Sends an `initialize` JSON-RPC request to the server and extracts
    /// the `Mcp-Session-Id` from the response. Returns a [`Connection`]
    /// that can be used to list/call tools and list/read resources.
    ///
    /// `headers` are forwarded on every request this connection makes
    /// to the upstream — both the initial `initialize` POST and every
    /// subsequent RPC. The client merges its own defaults
    /// (`User-Agent`, `X-Title`, `Referer`, `HTTP-Referer`) into this
    /// map, but caller-supplied values for any of those win on
    /// conflict. `Authorization` (when needed) is just another entry
    /// in `headers`. The `Mcp-Session-Id` header is reserved — pass it
    /// via `session_id` instead so the explicit argument can never be
    /// clobbered by the headers map.
    ///
    /// ## SSE handoff
    ///
    /// `Accept` is `text/event-stream, application/json` — stream first
    /// — so the server is encouraged to keep the underlying connection
    /// open. If the response comes back as SSE we read the initialize
    /// event off the stream and hand the *still-open* line reader to the
    /// returned [`Connection`]'s list-changed listener. The listener
    /// starts reading from that pre-opened stream immediately, which
    /// closes the race where a peer (e.g. an in-process rmcp upstream)
    /// would broadcast `notifications/tools/list_changed` before our
    /// listener had managed to open its own GET `/` SSE.
    ///
    /// If the response is unary JSON and the server advertises either
    /// `tools.list_changed` or `resources.list_changed`, we proactively
    /// open a GET `/` SSE stream *before returning* and hand it to the
    /// listener for the same reason. If neither capability is set, no
    /// listener is needed and we return without touching SSE.
    pub async fn connect(
        &self,
        url: String,
        session_id: Option<String>,
        headers: Option<IndexMap<String, String>>,
    ) -> Result<super::Connection, super::Error> {
        // Merge the caller's headers with the client's defaults once,
        // then reuse the same merged map across every retry of the
        // handshake AND hand it to the resulting Connection for every
        // later RPC. Caller-supplied headers always win over defaults.
        let headers = self.headers(headers);

        // One outer backoff retry around all three handshake steps —
        // initialize POST, notifications/initialized POST, GET / SSE
        // (when capabilities require it). On a failure of any step we
        // restart from scratch: a partial handshake leaves server-side
        // session state we can't reuse, so retrying just the failed
        // step would reference a session the server already discarded.
        // Every error is treated as transient — the loop only gives up
        // when the backoff's `max_elapsed_time` is exceeded.
        let mut backoff = backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            start_time: std::time::Instant::now(),
            max_elapsed_time: Some(self.backoff_max_elapsed_time),
            clock: backoff::SystemClock::default(),
        };

        loop {
            match self
                .connect_once(&url, session_id.as_deref(), &headers)
                .await
            {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    use backoff::backoff::Backoff;
                    match backoff.next_backoff() {
                        Some(d) => tokio::time::sleep(d).await,
                        None => return Err(e),
                    }
                }
            }
        }
    }

    /// Issue a stateless HTTP `DELETE /` to one upstream MCP server,
    /// telling it to terminate the session identified by `session_id`.
    ///
    /// This is the low-level primitive — no backoff, no listener
    /// teardown, no connection state. Caller is responsible for any
    /// retry semantics. For the stateful tear-down path that also
    /// cancels the connection's own active streams and absorbs
    /// upstream `404 / 401 / 403` as success, use
    /// [`Connection::delete`](super::Connection::delete) instead.
    ///
    /// `headers` is merged with the same defaults `connect` applies
    /// (`User-Agent`, `X-Title`, `Referer`, `HTTP-Referer`). The
    /// explicit `session_id` argument always wins over any
    /// `Mcp-Session-Id` entry that happens to appear in `headers` — the
    /// shape mirrors `connect`'s argument split for the same reason.
    ///
    /// Returns `Ok(())` on any 2xx status. Any non-2xx (including
    /// `404 Not Found`) surfaces as [`Error::BadStatus`]. Network /
    /// transport failures surface as [`Error::Request`].
    pub async fn delete(
        &self,
        url: String,
        session_id: String,
        headers: Option<IndexMap<String, String>>,
    ) -> Result<(), super::Error> {
        let headers = self.headers(headers);
        let mut request = apply_timeout(
            self.http_client.delete(&url),
            self.call_timeout,
        )
        .header("Mcp-Session-Id", &session_id);
        for (name, value) in &headers {
            // Explicit `session_id` arg always wins.
            if name.eq_ignore_ascii_case("Mcp-Session-Id") {
                continue;
            }
            request = request.header(name, value);
        }
        let response = request.send().await.map_err(|source| {
            super::Error::Request {
                url: url.clone(),
                source,
            }
        })?;
        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus {
                url,
                code,
                body: body.chars().take(800).collect(),
            });
        }
        Ok(())
    }

    /// One pass through the full Streamable-HTTP handshake. Caller
    /// applies the outer backoff retry loop in [`Self::connect`].
    /// `headers` is the already-merged map (defaults + caller overrides
    /// from [`Self::headers`]), reused on every request without further
    /// processing. `Mcp-Session-Id` is applied AFTER the headers loop
    /// so it always wins over any same-named entry in `headers`.
    async fn connect_once(
        &self,
        url: &str,
        session_id: Option<&str>,
        headers: &IndexMap<String, String>,
    ) -> Result<super::Connection, super::Error> {
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "objectiveai",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        });

        let mut request = apply_timeout(
            self.http_client.post(url),
            self.connect_timeout,
        )
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream, application/json")
        .json(&init_request);

        for (name, value) in headers {
            request = request.header(name, value);
        }
        // Mcp-Session-Id is applied last so the explicit `session_id`
        // argument always wins over any same-named entry in `headers`.
        if let Some(sid) = session_id {
            request = request.header("Mcp-Session-Id", sid);
        }

        let response = request.send().await.map_err(|source| {
            super::Error::Connection {
                url: url.to_string(),
                source,
            }
        })?;

        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus {
                url: url.to_string(),
                code,
                body,
            });
        }

        // Extract session ID from response header.
        //
        // For *new* sessions the server must mint and return a session
        // id. For *existing* sessions (caller passed `session_id` in)
        // many servers — including rmcp's `StreamableHttpService` on
        // its existing-session branch — don't echo the header back
        // because nothing changed. When the caller already knew the
        // session id, fall back to it instead of erroring; that's the
        // value we'll be stamping on every subsequent request anyway.
        let resolved_session_id = match response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
        {
            Some(s) => s,
            None => match session_id {
                Some(provided) => provided.to_string(),
                None => {
                    let body = response.text().await.unwrap_or_default();
                    return Err(super::Error::NoSessionId {
                        url: url.to_string(),
                        body: body.chars().take(800).collect(),
                    });
                }
            },
        };

        // Did the server return SSE or unary JSON? rmcp's
        // `StreamableHttpService` always returns SSE; many other servers
        // reply with bare JSON.
        let is_sse = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.starts_with("text/event-stream"))
            .unwrap_or(false);

        // Parse the initialize response. SSE path consumes one event
        // from the stream and keeps the rest of the stream alive for
        // the listener; unary path consumes the whole body.
        let (initialize_result, mut initial_sse_lines) = if is_sse {
            let mut lines = super::lines_from_response(response);
            let rpc_response: super::JsonRpcResponse<
                super::initialize_result::InitializeResult,
            > = super::read_next_sse_event(url, &mut lines).await?;
            let result = match rpc_response {
                super::JsonRpcResponse::Success { result, .. } => result,
                super::JsonRpcResponse::Error { error, .. } => {
                    return Err(super::Error::JsonRpc {
                        url: url.to_string(),
                        code: error.code,
                        message: error.message,
                        data: error.data,
                    });
                }
            };
            (result, Some(lines))
        } else {
            let rpc_response: super::JsonRpcResponse<
                super::initialize_result::InitializeResult,
            > = super::parse_streamable_http_response(url, response).await?;
            let result = match rpc_response {
                super::JsonRpcResponse::Success { result, .. } => result,
                super::JsonRpcResponse::Error { error, .. } => {
                    return Err(super::Error::JsonRpc {
                        url: url.to_string(),
                        code: error.code,
                        message: error.message,
                        data: error.data,
                    });
                }
            };
            (result, None)
        };

        // Whether we need a notification SSE channel at all.
        let needs_sse = initialize_result
            .capabilities
            .tools
            .as_ref()
            .and_then(|t| t.list_changed)
            .unwrap_or(false)
            || initialize_result
                .capabilities
                .resources
                .as_ref()
                .and_then(|r| r.list_changed)
                .unwrap_or(false);

        // Send `notifications/initialized` BEFORE any other request.
        // rmcp's per-session worker is in `expect_notification("initialized")`
        // at this point — anything else (a `tools/list`, an opportunistic
        // GET `/`) that lands during that window pushes a non-notification
        // through the worker, makes `serve_server_with_ct_inner` return
        // `Err(ExpectedInitializedNotification(...))`, drops the
        // WorkerTransport, cancels the worker via its drop_guard, and
        // tears the whole session down. Every later POST then 500s with
        // "Session service terminated."
        //
        // We don't have a `Connection` yet — building one would spawn
        // `refresh_tools` / `refresh_resources` background tasks that
        // race with this notification, which is exactly the bug we're
        // avoiding. We therefore POST inline here.
        let init_notification_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {},
        });
        let mut notify_request = apply_timeout(
            self.http_client.post(url),
            self.call_timeout,
        )
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
        for (name, value) in headers {
            notify_request = notify_request.header(name, value);
        }
        notify_request =
            notify_request.header("Mcp-Session-Id", &resolved_session_id);
        let notify_response = notify_request
            .json(&init_notification_body)
            .send()
            .await
            .map_err(|source| super::Error::Request {
                url: url.to_string(),
                source,
            })?;
        if !notify_response.status().is_success() {
            let code = notify_response.status();
            let body = notify_response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus {
                url: url.to_string(),
                code,
                body,
            });
        }

        // Now safe to drop the init-side SSE stream; rmcp's session
        // worker is past the init handshake and we have no further use
        // for it (notifications come on the GET / stream below).
        drop(initial_sse_lines.take());

        // Now that the server's session worker is past the
        // `expect_notification` gate, it's safe to open the proactive
        // GET `/` SSE stream the listener will read
        // `notifications/{tools,resources}/list_changed` from. Capability
        // inspection only gates whether we open this stream; the
        // `Connection` itself is naive about capabilities.
        let initial_sse_lines: Option<super::LinesStream> = if needs_sse {
            let mut get_request = apply_timeout(
                self.http_client.get(url),
                self.connect_timeout,
            )
            .header("Accept", "text/event-stream");
            for (name, value) in headers {
                get_request = get_request.header(name, value);
            }
            get_request =
                get_request.header("Mcp-Session-Id", &resolved_session_id);
            let get_response = get_request.send().await.map_err(|source| {
                super::Error::Connection {
                    url: url.to_string(),
                    source,
                }
            })?;
            if !get_response.status().is_success() {
                let code = get_response.status();
                let body = get_response.text().await.unwrap_or_default();
                return Err(super::Error::BadStatus {
                    url: url.to_string(),
                    code,
                    body,
                });
            }
            Some(super::lines_from_response(get_response))
        } else {
            None
        };

        // Construct the Connection at the very end. This is the only
        // place in `connect` where the listener task and the
        // `refresh_tools` / `refresh_resources` background tasks get
        // spawned — by now the upstream is fully past its init
        // handshake, so any of those POSTs land safely.
        let connection = super::Connection::new(
            self.http_client.clone(),
            url.to_string(),
            resolved_session_id,
            headers.clone(),
            self.backoff_current_interval,
            self.backoff_initial_interval,
            self.backoff_randomization_factor,
            self.backoff_multiplier,
            self.backoff_max_interval,
            self.backoff_max_elapsed_time,
            self.call_timeout,
            initialize_result,
            initial_sse_lines,
        )
        .await;

        Ok(connection)
    }
}
