//! HTTP client implementation for ObjectiveAI API.

use crate::error;
use eventsource_stream::Event as MessageEvent;
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, RequestBuilderExt};

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
///     None, // Use default API base
///     Some("your-api-key"),
///     None, // user_agent
///     None, // x_title
///     None, // referer
/// );
/// ```
#[derive(Debug, Clone)]
pub struct HttpClient {
    /// The underlying reqwest HTTP client.
    pub http_client: reqwest::Client,
    /// Base URL for API requests. Defaults to `https://api.objective-ai.io`.
    pub api_base: String,
    /// API key for authentication. Sent as `Bearer` token in `Authorization` header.
    pub api_key: Option<String>,
    /// Value for the `User-Agent` header.
    pub user_agent: Option<String>,
    /// Value for the `X-Title` header.
    pub x_title: Option<String>,
    /// Value for both `Referer` and `HTTP-Referer` headers.
    pub referer: Option<String>,
}

impl HttpClient {
    /// Creates a new HTTP client.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The reqwest client to use for requests
    /// * `api_base` - Base URL for API requests (defaults to `https://api.objective-ai.io`)
    /// * `api_key` - API key for authentication
    /// * `user_agent` - Optional User-Agent header value
    /// * `x_title` - Optional X-Title header value
    /// * `referer` - Optional Referer header value
    pub fn new(
        http_client: reqwest::Client,
        api_base: Option<impl Into<String>>,
        api_key: Option<impl Into<String>>,
        user_agent: Option<impl Into<String>>,
        x_title: Option<impl Into<String>>,
        referer: Option<impl Into<String>>,
    ) -> Self {
        Self {
            http_client,
            api_base: match api_base {
                Some(base) => base.into(),
                None => "https://api.objective-ai.io".to_string(),
            },
            api_key: api_key.map(Into::into),
            user_agent: user_agent.map(Into::into),
            x_title: x_title.map(Into::into),
            referer: referer.map(Into::into),
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
            self.api_base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut request = self.http_client.request(method, &url);
        if let Some(api_key) = &self.api_key {
            request =
                request.header("authorization", format!("Bearer {}", api_key));
        }
        if let Some(user_agent) = &self.user_agent {
            request = request.header("user-agent", user_agent);
        }
        if let Some(x_title) = &self.x_title {
            request = request.header("x-title", x_title);
        }
        if let Some(referer) = &self.referer {
            request = request.header("referer", referer);
            request = request.header("http-referer", referer);
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
    /// # Arguments
    ///
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `path` - API endpoint path (will be appended to `api_base`)
    /// * `body` - Optional request body to serialize as JSON
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
    /// # Arguments
    ///
    /// * `method` - HTTP method (GET, POST, DELETE, etc.)
    /// * `path` - API endpoint path (will be appended to `api_base`)
    /// * `body` - Optional request body to serialize as JSON
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
    /// # Arguments
    ///
    /// * `method` - HTTP method (typically POST for streaming)
    /// * `path` - API endpoint path (will be appended to `api_base`)
    /// * `body` - Optional request body to serialize as JSON
    ///
    /// # Errors
    ///
    /// Returns [`super::HttpError`] if the stream cannot be established. Individual
    /// stream items may also contain errors if chunks fail to deserialize.
    pub async fn send_streaming<
        T: serde::de::DeserializeOwned + Send + 'static,
    >(
        &self,
        method: reqwest::Method,
        path: impl AsRef<str>,
        body: Option<impl serde::Serialize>,
    ) -> Result<
        impl Stream<Item = Result<T, super::HttpError>> + Send + 'static,
        super::HttpError,
    > {
        Ok(
            self.request(method, path.as_ref(), body)
                .eventsource()?
                .then(|result| async {
                    match result {
                        Ok(Event::Open) => None,
                        Ok(Event::Message(MessageEvent { data, .. }))
                            if data == "[DONE]"
                                || data.starts_with(":")
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
