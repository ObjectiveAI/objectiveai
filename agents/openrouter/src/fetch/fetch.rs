//! One OpenRouter call: the request out, the chunk stream back.

use std::collections::HashMap;

use crate::agent::Agent;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use eventsource_stream::Event as MessageEvent;
use rmcp::model::ContentBlock;
use futures_util::{Stream, StreamExt as _};
use reqwest_eventsource::{Event, RequestBuilderExt as _};

use crate::continuation::Continuation;
use crate::stream_once::StreamOnce;
use super::{Error, ProviderError};
use crate::request::ChatCompletionCreateParams;
use crate::response::ChatCompletionChunk;

/// Where the requests go.
const ADDRESS: &str = "https://openrouter.ai/api/v1";

/// Call OpenRouter and stream the answer back as loop chunks.
///
/// The request is built whole from the three arguments — agent
/// parameters, the opened continuation, this turn's message — and the
/// answer arrives as SSE, each event decoded and folded into the
/// protocol's chunk vocabulary by
/// [`ChatCompletionChunk::into_chunks`].
///
/// # The first item is never an error
///
/// The rule the api crate's upstreams learned: a stream whose FIRST
/// item would be an error is not a stream, it is a failure to start —
/// so the first item is awaited here, a leading error returns `Err`,
/// and a stream with nothing on it at all returns
/// [`Error::EmptyStream`]. Errors after the first chunk arrive
/// mid-stream as `Err` items, and end the stream.
///
pub async fn fetch(
    api_key: &str,
    agent: Agent,
    continuation: Option<Continuation>,
    content: Vec<ContentBlock>,
    tools: Option<Vec<crate::request::Tool>>,
) -> Result<
    // `use<>`: the stream borrows nothing from the arguments — the
    // key is spent on a header before the stream exists.
    impl Stream<Item = Result<AgenticLoopChunk, Error>> + Send + Unpin + use<>,
    Error,
> {
    let request =
        ChatCompletionCreateParams::new(agent, continuation, content, tools);

    let event_source = reqwest::Client::new()
        .post(format!("{ADDRESS}/chat/completions"))
        .header("authorization", format!("Bearer {api_key}"))
        .header(
            "user-agent",
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .header("x-title", "Diverge")
        .header("referer", "https://diverge.network")
        .header("http-referer", "https://diverge.network")
        .json(&request)
        .eventsource()
        .expect("the request body is JSON, which clones");

    let inner = stream(event_source);

    // Await the first item, so a failure to start is a `Result` and
    // never a stream's leading item. The item taken is re-emitted in
    // front of the rest; nothing is lost.
    let mut rest = Box::pin(inner);
    match rest.next().await {
        Some(Ok(first)) => Ok(StreamOnce::new(Ok(first)).chain(rest)),
        Some(Err(error)) => Err(error),
        None => Err(Error::EmptyStream),
    }
}

/// The SSE loop: every event decoded, every hard-won branch kept.
///
/// The branches below were learned at runtime in the api crate and are
/// preserved exactly: the `[DONE]` sentinel ends the stream; comment
/// and empty data lines are skipped; a data event that does not parse
/// as a chunk is tried as a PROVIDER error before being reported as a
/// deserialization failure, because OpenRouter delivers provider
/// errors as ordinary data events; a bad status becomes the body,
/// JSON if possible; and any error ends the stream.
fn stream(
    mut event_source: reqwest_eventsource::EventSource,
) -> impl Stream<Item = Result<AgenticLoopChunk, Error>> + Send {
    async_stream::stream! {
        let mut tool_calls: HashMap<u64, (String, String)> = HashMap::new();
        let mut buffer: Vec<AgenticLoopChunk> = Vec::new();

        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => continue,
                Ok(Event::Message(MessageEvent { data, .. })) => {
                    if data == "[DONE]" {
                        break;
                    } else if data.starts_with(":") {
                        continue;
                    } else if data.is_empty() {
                        continue;
                    }
                    let mut deserializer =
                        serde_json::Deserializer::from_str(&data);
                    match serde_path_to_error::deserialize::<
                        _,
                        ChatCompletionChunk,
                    >(&mut deserializer)
                    {
                        Ok(chunk) => {
                            chunk.into_chunks(&mut tool_calls, &mut buffer);
                            for chunk in buffer.drain(..) {
                                yield Ok(chunk);
                            }
                        }
                        Err(error) => {
                            let mut retry =
                                serde_json::Deserializer::from_str(&data);
                            match serde_path_to_error::deserialize::<
                                _,
                                ProviderError,
                            >(&mut retry)
                            {
                                Ok(provider) => {
                                    yield Err(Error::Provider(provider));
                                }
                                Err(_) => {
                                    yield Err(Error::Deserialization(error));
                                }
                            }
                            break;
                        }
                    }
                }
                Err(reqwest_eventsource::Error::InvalidStatusCode(
                    code,
                    response,
                )) => {
                    let body = match response.text().await {
                        Ok(body) => {
                            match serde_json::from_str::<serde_json::Value>(
                                &body,
                            ) {
                                Ok(value) => value,
                                Err(_) => serde_json::Value::String(body),
                            }
                        }
                        Err(_) => serde_json::Value::Null,
                    };
                    yield Err(Error::BadStatus { code, body });
                    break;
                }
                Err(error) => {
                    yield Err(Error::Stream(error));
                    break;
                }
            }
        }
    }
}
