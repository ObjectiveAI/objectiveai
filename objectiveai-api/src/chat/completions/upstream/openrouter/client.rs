//! OpenRouter HTTP client implementation.

use eventsource_stream::Event as MessageEvent;
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use std::time::Duration;

/// Generates a unique response ID for a chat completion.
///
/// Combines a UUID with the creation timestamp for uniqueness.
pub fn response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("chtcpl-{}-{}", uuid.simple(), created)
}

/// HTTP client for communicating with the OpenRouter API.
#[derive(Debug, Clone)]
pub struct Client {
    /// The underlying HTTP client.
    pub http_client: reqwest::Client,
    /// Base URL for the OpenRouter API.
    pub api_base: String,
    /// API key for authentication with OpenRouter.
    pub api_key: String,
    /// Optional User-Agent header value.
    pub user_agent: Option<String>,
    /// Optional X-Title header value.
    pub x_title: Option<String>,
    /// Optional Referer header value (sent as both referer and http-referer).
    pub referer: Option<String>,
}

impl Client {
    /// Creates a new OpenRouter client.
    pub fn new(
        http_client: reqwest::Client,
        api_base: String,
        api_key: String,
        user_agent: Option<String>,
        x_title: Option<String>,
        referer: Option<String>,
    ) -> Self {
        Self {
            http_client,
            api_base,
            api_key,
            user_agent,
            x_title,
            referer,
        }
    }

    /// Creates a streaming chat completion request.
    ///
    /// Transforms the request using the Ensemble LLM's configuration and
    /// returns a stream of chat completion chunks.
    pub fn create_streaming_for_chat(
        &self,
        id: String,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        first_chunk_timeout: Duration,
        other_chunk_timeout: Duration,
        ensemble_llm: &objectiveai::ensemble_llm::EnsembleLlm,
        request: &objectiveai::chat::completions::request::ChatCompletionCreateParams,
    ) -> impl Stream<
        Item = Result<
            objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            super::Error,
        >,
    > + Send
    + 'static {
        self.create_streaming(
            id,
            ensemble_llm.id.clone(),
            byok,
            cost_multiplier,
            first_chunk_timeout,
            other_chunk_timeout,
            super::request::ChatCompletionCreateParams::new_for_chat(ensemble_llm, request),
        )
    }

    /// Creates a streaming chat completion for LLM voting in vector completions.
    ///
    /// The LLM sees responses labeled with prefix keys (e.g., `` `A` ``) and responds
    /// with its choice. The `vector_pfx_indices` maps the prefix keys shown to the LLM
    /// to the indices of the responses in the original request.
    pub fn create_streaming_for_vector(
        &self,
        id: String,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        first_chunk_timeout: Duration,
        other_chunk_timeout: Duration,
        ensemble_llm: &objectiveai::ensemble_llm::EnsembleLlm,
        request: &objectiveai::vector::completions::request::VectorCompletionCreateParams,
        vector_pfx_indices: &[(String, usize)],
    ) -> impl Stream<
        Item = Result<
            objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            super::Error,
        >,
    > + Send
    + 'static {
        self.create_streaming(
            id,
            ensemble_llm.id.clone(),
            byok,
            cost_multiplier,
            first_chunk_timeout,
            other_chunk_timeout,
            super::request::ChatCompletionCreateParams::new_for_vector(
                vector_pfx_indices,
                ensemble_llm,
                request,
            ),
        )
    }

    /// Internal method that creates the streaming request to OpenRouter.
    fn create_streaming(
        &self,
        id: String,
        model: String,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        first_chunk_timeout: Duration,
        other_chunk_timeout: Duration,
        request: super::request::ChatCompletionCreateParams,
    ) -> impl Stream<
        Item = Result<
            objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            super::Error,
        >,
    > + Send
    + 'static {
        let is_byok = byok.is_some();
        let event_source =
            self.create_streaming_event_source(byok.unwrap_or(&self.api_key), &request);
        Self::create_streaming_stream(
            event_source,
            id,
            model,
            is_byok,
            cost_multiplier,
            first_chunk_timeout,
            other_chunk_timeout,
        )
    }

    /// Creates an SSE EventSource for the streaming request.
    fn create_streaming_event_source(
        &self,
        api_key: &str,
        request: &super::request::ChatCompletionCreateParams,
    ) -> EventSource {
        let mut http_request = self
            .http_client
            .post(format!("{}/chat/completions", self.api_base))
            .header("authorization", format!("Bearer {}", api_key));
        if let Some(ref user_agent) = self.user_agent {
            http_request = http_request.header("user-agent", user_agent);
        }
        if let Some(ref x_title) = self.x_title {
            http_request = http_request.header("x-title", x_title);
        }
        if let Some(ref referer) = self.referer {
            http_request = http_request
                .header("referer", referer)
                .header("http-referer", referer);
        }
        http_request.json(request).eventsource().unwrap()
    }

    /// Processes the SSE EventSource into a stream of chat completion chunks.
    ///
    /// Handles timeouts, error responses, and transforms upstream chunks to downstream format.
    fn create_streaming_stream(
        mut event_source: EventSource,
        id: String,
        model: String,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
        first_chunk_timeout: Duration,
        other_chunk_timeout: Duration,
    ) -> impl Stream<
        Item = Result<
            objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            super::Error,
        >,
    > + Send
    + 'static {
        async_stream::stream! {
            let mut first = true;
            while let Some(event) = tokio::time::timeout(
                if first {
                    first_chunk_timeout
                } else {
                    other_chunk_timeout
                },
                event_source.next(),
            ).await.transpose() {
                first = false;
                match event {
                    Ok(Ok(Event::Open)) => continue,
                    Ok(Ok(Event::Message(MessageEvent { data, .. }))) => {
                        if data == "[DONE]" {
                            break;
                        } else if data.starts_with(":") {
                            continue; // skip comments
                        } else if data.is_empty() {
                            continue; // skip empty messages
                        }
                        let mut de = serde_json::Deserializer::from_str(&data);
                        match serde_path_to_error::deserialize::<
                            _,
                            super::response::ChatCompletionChunk,
                        >(&mut de)
                        {
                            Ok(chunk) => yield Ok(chunk.into_downstream(
                                id.clone(),
                                model.clone(),
                                is_byok,
                                cost_multiplier,
                            )),
                            Err(e) => {
                                de = serde_json::Deserializer::from_str(&data);
                                match serde_path_to_error::deserialize::<
                                    _,
                                    super::OpenRouterProviderError,
                                >(&mut de)
                                {
                                    Ok(provider_error) => yield Err(
                                        super::Error::OpenRouterProviderError(
                                            provider_error,
                                        ),
                                    ),
                                    Err(_) => yield Err(
                                        super::Error::DeserializationError(e),
                                    ),
                                }
                            }
                        }
                    }
                    Ok(Err(reqwest_eventsource::Error::InvalidStatusCode(
                        code,
                        response,
                    ))) => {
                        match response.text().await {
                            Ok(body) => {
                                yield Err(super::Error::BadStatus {
                                    code,
                                    body: match serde_json::from_str::<
                                        serde_json::Value,
                                    >(
                                        &body,
                                    ) {
                                        Ok(value) => value,
                                        Err(_) => serde_json::Value::String(
                                            body,
                                        ),
                                    },
                                });
                            }
                            Err(_) => {
                                yield Err(super::Error::BadStatus {
                                    code,
                                    body: serde_json::Value::Null,
                                });
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        yield Err(super::Error::from(e));
                    }
                    Err(_) => {
                        yield Err(super::Error::StreamTimeout);
                    }
                }
            }
        }
    }
}
