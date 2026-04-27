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
    pub connect_timeout: Duration,

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
    pub call_timeout: Duration,
}

impl Client {
    /// Creates a new MCP client.
    pub fn new(
        http_client: reqwest::Client,
        user_agent: String,
        x_title: String,
        http_referer: String,
        connect_timeout: Duration,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Duration,
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

    /// Connects to an MCP server using the Streamable HTTP transport.
    ///
    /// Sends an `initialize` JSON-RPC request to the server and extracts
    /// the `Mcp-Session-Id` from the response. Returns a [`Connection`]
    /// that can be used to list/call tools and list/read resources.
    ///
    /// `extra_headers` are forwarded on every request this connection
    /// makes to the upstream — both the initial `initialize` POST and
    /// every subsequent RPC. They are applied *after* the fixed headers
    /// so callers can't accidentally clobber `Mcp-Session-Id`,
    /// `Content-Type`, etc.
    pub async fn connect(
        &self,
        url: String,
        authorization: Option<String>,
        session_id: Option<String>,
        extra_headers: IndexMap<String, String>,
    ) -> Result<super::Connection, super::Error> {
        if url == "mock" {
            return Ok(super::Connection::new_mock(url));
        }

        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "objectiveai",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        });

        let mut request = self
            .http_client
            .post(&url)
            .timeout(self.connect_timeout)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&init_request);

        if let Some(sid) = &session_id {
            request = request.header("Mcp-Session-Id", sid);
        }
        if let Some(auth) = &authorization {
            request = request.header("Authorization", auth);
        }
        request = request.header("User-Agent", &self.user_agent);
        request = request.header("X-Title", &self.x_title);
        request = request.header("Referer", &self.http_referer);
        request = request.header("HTTP-Referer", &self.http_referer);
        for (name, value) in &extra_headers {
            request = request.header(name, value);
        }

        let response =
            request.send().await.map_err(super::Error::Connection)?;

        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus { code, body });
        }

        // Extract session ID from response header.
        let session_id = response
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or(super::Error::NoSessionId)?;

        // Parse the initialize result.
        let rpc_response: super::JsonRpcResponse<
            super::initialize_result::InitializeResult,
        > = response.json().await.map_err(super::Error::Request)?;

        let initialize_result = match rpc_response {
            super::JsonRpcResponse::Success { result, .. } => result,
            super::JsonRpcResponse::Error { error, .. } => {
                return Err(super::Error::JsonRpc {
                    code: error.code,
                    message: error.message,
                    data: error.data,
                });
            }
        };

        let connection = super::Connection::new(
            self.http_client.clone(),
            url,
            session_id,
            authorization,
            self.user_agent.clone(),
            self.x_title.clone(),
            self.http_referer.clone(),
            extra_headers,
            self.backoff_current_interval,
            self.backoff_initial_interval,
            self.backoff_randomization_factor,
            self.backoff_multiplier,
            self.backoff_max_interval,
            self.backoff_max_elapsed_time,
            self.call_timeout,
            initialize_result,
        );

        // Send the initialized notification.
        connection
            .notify("initialized", &serde_json::json!({}))
            .await?;

        Ok(connection)
    }
}
