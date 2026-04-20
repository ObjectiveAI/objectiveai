//! HTTP client implementation for ObjectiveAI API.

use crate::error;
use eventsource_stream::Event as MessageEvent;
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, RequestBuilderExt};
use std::sync::Arc;

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
///     None, // x_viewer_signature
///     None, // x_viewer_address
///     None, // x_commit_author_name
///     None, // x_commit_author_email
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
    pub x_mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
    /// Value for the `X-VIEWER-SIGNATURE` header.
    pub x_viewer_signature: Option<Arc<String>>,
    /// Value for the `X-VIEWER-ADDRESS` header.
    pub x_viewer_address: Option<Arc<String>>,
    /// Value for the `X-COMMIT-AUTHOR-NAME` header.
    pub x_commit_author_name: Option<Arc<String>>,
    /// Value for the `X-COMMIT-AUTHOR-EMAIL` header.
    pub x_commit_author_email: Option<Arc<String>>,
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
    /// * `x_viewer_signature` - Optional X-VIEWER-SIGNATURE header value
    /// * `x_viewer_address` - Optional X-VIEWER-ADDRESS header value
    /// * `x_commit_author_name` - Optional X-COMMIT-AUTHOR-NAME header value
    /// * `x_commit_author_email` - Optional X-COMMIT-AUTHOR-EMAIL header value
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
        x_viewer_signature: Option<impl Into<String>>,
        x_viewer_address: Option<impl Into<String>>,
        x_commit_author_name: Option<impl Into<String>>,
        x_commit_author_email: Option<impl Into<String>>,
    ) -> Self {
        #[cfg(feature = "env")]
        let env = |name: &str| -> Option<String> { std::env::var(name).ok() };

        Self {
            http_client,
            address: match address {
                Some(base) => base.into(),
                #[cfg(feature = "env")]
                None => env("OBJECTIVEAI_ADDRESS")
                    .unwrap_or_else(|| "https://api.objectiveai.dev".to_string()),
                #[cfg(not(feature = "env"))]
                None => "https://api.objectiveai.dev".to_string(),
            },
            authorization: authorization.map(|k| Arc::new(k.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("OBJECTIVEAI_AUTHORIZATION").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            user_agent: user_agent.map(Into::into)
                .or_else(|| { #[cfg(feature = "env")] { env("USER_AGENT") } #[cfg(not(feature = "env"))] { None } }),
            x_title: x_title.map(Into::into)
                .or_else(|| { #[cfg(feature = "env")] { env("X_TITLE") } #[cfg(not(feature = "env"))] { None } }),
            http_referer: http_referer.map(Into::into)
                .or_else(|| { #[cfg(feature = "env")] { env("HTTP_REFERER") } #[cfg(not(feature = "env"))] { None } }),
            x_github_authorization: x_github_authorization.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("GITHUB_AUTHORIZATION").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_openrouter_authorization: x_openrouter_authorization.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("OPENROUTER_AUTHORIZATION").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_mcp_authorization: x_mcp_authorization.map(Arc::new)
                .or_else(|| { #[cfg(feature = "env")] { env("MCP_AUTHORIZATION").and_then(|v| serde_json::from_str(&v).ok()).map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_viewer_signature: x_viewer_signature.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("VIEWER_SIGNATURE").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_viewer_address: x_viewer_address.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("VIEWER_ADDRESS").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_commit_author_name: x_commit_author_name.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("COMMIT_AUTHOR_NAME").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
            x_commit_author_email: x_commit_author_email.map(|v| Arc::new(v.into()))
                .or_else(|| { #[cfg(feature = "env")] { env("COMMIT_AUTHOR_EMAIL").map(Arc::new) } #[cfg(not(feature = "env"))] { None } }),
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
            let key = authorization.strip_prefix("Bearer ").unwrap_or(authorization);
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
            request = request.header("X-OPENROUTER-AUTHORIZATION", token.as_str());
        }
        if let Some(headers) = &self.x_mcp_authorization {
            if let Ok(json) = serde_json::to_string(headers.as_ref()) {
                request = request.header("X-MCP-AUTHORIZATION", json);
            }
        }
        if let Some(sig) = &self.x_viewer_signature {
            request = request.header("X-VIEWER-SIGNATURE", sig.as_str());
        }
        if let Some(addr) = &self.x_viewer_address {
            request = request.header("X-VIEWER-ADDRESS", addr.as_str());
        }
        if let Some(name) = &self.x_commit_author_name {
            request = request.header("X-COMMIT-AUTHOR-NAME", name.as_str());
        }
        if let Some(email) = &self.x_commit_author_email {
            request = request.header("X-COMMIT-AUTHOR-EMAIL", email.as_str());
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
        Ok(
            self.request(method, path.as_ref(), body)
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
}
