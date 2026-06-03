//! OpenRouter HTTP client for agent completions.

use crate::agent::completions::{
    ContinuationItem, StreamItem, UpstreamClient,
};
use crate::util::StreamOnce;
use eventsource_stream::Event as MessageEvent;
use futures::{Stream, StreamExt};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use std::pin::Pin;
use std::sync::Arc;

/// HTTP client for communicating with the OpenRouter API for agent completions.
#[derive(Debug, Clone)]
pub struct Client {
    /// The underlying HTTP client.
    pub http_client: reqwest::Client,
    /// Base URL for the OpenRouter API.
    pub address: String,
    /// API key for authentication with OpenRouter.
    pub authorization: Option<String>,
    /// User-Agent header value.
    pub user_agent: String,
    /// X-Title header value.
    pub x_title: String,
    /// Referer header value (sent as both referer and http-referer).
    pub http_referer: String,
}

impl Client {
    pub fn new(
        http_client: reqwest::Client,
        address: String,
        authorization: Option<String>,
        user_agent: String,
        x_title: String,
        http_referer: String,
    ) -> Self {
        Self { http_client, address, authorization, user_agent, x_title, http_referer }
    }

    /// Creates an SSE EventSource for the streaming request.
    fn create_streaming_event_source(
        &self,
        api_key: &str,
        request: &super::request::ChatCompletionCreateParams,
    ) -> EventSource {
        let mut http_request = self
            .http_client
            .post(format!("{}/chat/completions", self.address))
            .header("authorization", if api_key.starts_with("Bearer ") {
                api_key.to_string()
            } else {
                format!("Bearer {}", api_key)
            })
            .header("user-agent", &self.user_agent)
            .header("x-title", &self.x_title)
            .header("referer", &self.http_referer)
            .header("http-referer", &self.http_referer);
        http_request.json(request).eventsource().unwrap()
    }

    /// Processes the SSE EventSource into a stream of agent completion chunks
    /// (or typed errors), followed by a final accumulated state.
    #[allow(clippy::too_many_arguments)]
    fn create_streaming_stream(
        mut event_source: EventSource,
        id: String,
        created: u64,
        index: u64,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> impl Stream<
        Item = Result<
            StreamItem<
                objectiveai_sdk::agent::completions::message::AssistantMessage,
            >,
            super::Error,
        >,
    > + Send
    + 'static {
        async_stream::stream! {
            use objectiveai_sdk::agent::completions::message::AssistantMessage;
            use objectiveai_sdk::agent::completions::response::streaming::{
                AssistantResponseChunk, MessageChunk,
            };

            let mut accumulated: Option<AssistantResponseChunk> = None;
            let mut had_error = false;

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
                        let mut de =
                            serde_json::Deserializer::from_str(&data);
                        match serde_path_to_error::deserialize::<
                            _,
                            super::response::ChatCompletionChunk,
                        >(&mut de)
                        {
                            Ok(chunk) => {
                                let downstream = chunk.into_downstream(
                                    id.clone(),
                                    created,
                                    index,
                                    is_byok,
                                    cost_multiplier,
                                    agent_instance_hierarchy.clone(),
                                    agent_id.clone(),
                                    agent_full_id.clone(),
                                    agent_remote.clone(),
                                );

                                // Accumulate the assistant response chunk.
                                for message in &downstream.messages {
                                    if let MessageChunk::Assistant(
                                        assistant_chunk,
                                    ) = message
                                    {
                                        match &mut accumulated {
                                            Some(acc) => {
                                                acc.push(assistant_chunk)
                                            }
                                            None => {
                                                accumulated = Some(
                                                    assistant_chunk.clone(),
                                                )
                                            }
                                        }
                                    }
                                }

                                yield Ok(StreamItem::Chunk(downstream));
                            }
                            Err(e) => {
                                // Try to parse as a provider error,
                                // otherwise report as deserialization error.
                                let mut de2 =
                                    serde_json::Deserializer::from_str(&data);
                                match serde_path_to_error::deserialize::<
                                    _,
                                    super::OpenRouterProviderError,
                                >(&mut de2)
                                {
                                    Ok(provider_error) => {
                                        yield Err(
                                            super::Error::OpenRouterProviderError(
                                                provider_error,
                                            ),
                                        );
                                    }
                                    Err(_) => {
                                        yield Err(
                                            super::Error::DeserializationError(e),
                                        );
                                    }
                                }
                                had_error = true;
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
                                match serde_json::from_str::<
                                    serde_json::Value,
                                >(
                                    &body,
                                ) {
                                    Ok(value) => value,
                                    Err(_) => {
                                        serde_json::Value::String(body)
                                    }
                                }
                            }
                            Err(_) => serde_json::Value::Null,
                        };
                        yield Err(super::Error::BadStatus { code, body });
                        had_error = true;
                        break;
                    }
                    Err(e) => {
                        yield Err(super::Error::StreamError(e));
                        had_error = true;
                        break;
                    }
                }
            }

            if !had_error {
                // Yield the final accumulated state.
                let state = match accumulated {
                    Some(acc) => AssistantMessage {
                        content: acc.content,
                        name: None,
                        refusal: acc.refusal,
                        tool_calls: acc.tool_calls.map(|tcs| {
                            tcs.into_iter().map(Into::into).collect()
                        }),
                        reasoning: acc.reasoning,
                    },
                    None => AssistantMessage {
                        content: None,
                        name: None,
                        refusal: None,
                        tool_calls: None,
                        reasoning: None,
                    },
                };
                yield Ok(StreamItem::State(state));
            }
        }
    }
}

impl UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation> for Client {
    type State = objectiveai_sdk::agent::completions::message::AssistantMessage;
    type Stream = Pin<
        Box<dyn Stream<Item = StreamItem<Self::State>> + Send + 'static>,
    >;
    type Error = super::Error;

    fn create(
        &self,
        id: &str,
        created: u64,
        agent: &objectiveai_sdk::agent::openrouter::Agent,
        request_continuation: Option<&objectiveai_sdk::agent::openrouter::Continuation>,
        params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        continuation: Option<&[ContinuationItem<Self::State>]>,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        _tools_enabled: bool,
        _invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        _invention_step: Option<usize>,
        _invention_tasks_min: Option<u64>,
        _invention_input_schema: Option<String>,
        agent_instance_hierarchy: &str,
        agent_id: &str,
        agent_full_id: &str,
        agent_remote: Option<&objectiveai_sdk::RemotePath>,
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static {
        let tools_enabled = _tools_enabled;
        let id = id.to_string();
        let agent = agent.clone();
        let params = params.clone();
        let messages = messages.to_vec();
        let continuation = continuation.map(|c| c.to_vec());
        let request_continuation = request_continuation.cloned();
        let client = self.clone();
        let agent_instance_hierarchy = agent_instance_hierarchy.to_string();
        let agent_id = agent_id.to_string();
        let agent_full_id = agent_full_id.to_string();
        let agent_remote = agent_remote.cloned();
        let is_byok = byok.is_some();
        let byok = byok.map(String::from);

        async move {
            // Reject required tool call when tools are not allowed.
            if !tools_enabled {
                use objectiveai_sdk::agent::completions::request::{ResponseFormat, ResponseFormatParam};
                let resolved_rf = match params.response_format.as_ref() {
                    Some(ResponseFormatParam::Single(rf)) => Some(rf),
                    Some(ResponseFormatParam::PerAgent(map)) => map.get(&agent.id),
                    None => None,
                };
                if let Some(ResponseFormat::ToolCall { required: Some(true), .. }) = resolved_rf {
                    return Err(super::Error::ToolsNotAllowedWithRequiredToolCall);
                }
            }

            use objectiveai_sdk::agent::completions::request::ResponseFormatParam;
            let response_format = match params.response_format.as_ref() {
                Some(ResponseFormatParam::Single(rf)) => Some(rf.clone()),
                Some(ResponseFormatParam::PerAgent(map)) => map.get(&agent.id).cloned(),
                None => None,
            };
            // Source MCP tools from the per-agent proxy connection (if any).
            // The proxy fans out to the agent's declared upstream MCP
            // servers and the invention server, so a single list_tools()
            // (inside resolve_tools) returns the union — no separate
            // invention_tools path.
            let (tool_names, tool_map) = super::super::resolved_tool::resolve_tools(
                mcp_connection.as_ref(),
                response_format.as_ref(),
            )
            .await
            .map_err(|e| super::Error::Mcp {
                url: e.url,
                error: e.error,
            })?;

            let request =
                super::request::ChatCompletionCreateParams::new(
                    &agent,
                    &params,
                    &messages,
                    continuation.as_deref(),
                    request_continuation.as_ref(),
                    &tool_names,
                    &tool_map,
                    tools_enabled,
                );

            let api_key = match byok.as_deref().or(client.authorization.as_deref()) {
                Some(key) => key,
                None => return Err(super::Error::MissingApiKey),
            };
            let event_source =
                client.create_streaming_event_source(api_key, &request);

            let index = continuation
                .as_deref()
                .map(|c| {
                    c.iter()
                        .filter(|item| {
                            matches!(
                                item,
                                ContinuationItem::State(_)
                                    | ContinuationItem::ToolMessage(_)
                            )
                        })
                        .count() as u64
                })
                .unwrap_or(0);

            let internal_stream = Self::create_streaming_stream(
                event_source,
                id.clone(),
                created,
                index,
                is_byok,
                cost_multiplier,
                agent_instance_hierarchy.clone(),
                agent_id.clone(),
                agent_full_id.clone(),
                agent_remote.clone(),
            );


            // Await the first stream item. If it is an error,
            // return Err so the caller never sees an error as the
            // first yielded item.
            let mut stream = Box::pin(internal_stream);
            match stream.next().await {
                Some(Err(e)) => {
                    return Err(e);
                }
                Some(Ok(first)) => {
                    // Map the remaining internal stream: typed errors become
                    // error chunks for mid-stream delivery to the client.
                    let id_for_stream = id.clone();
                    let agent_instance_hierarchy_for_stream = agent_instance_hierarchy.clone();
                    let agent_id_for_stream = agent_id.clone();
                    let agent_full_id_for_stream = agent_full_id.clone();
                    let agent_remote_for_stream = agent_remote.clone();
                    let rest = stream.map(move |item| match item {
                        Ok(si) => si,
                        Err(e) => {
                            use objectiveai_sdk::error::StatusError;
                            StreamItem::Chunk(
                                objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                                    id: id_for_stream.clone(),
                                    agent_instance_hierarchy: agent_instance_hierarchy_for_stream.clone(),
                                    agent_id: agent_id_for_stream.clone(),
                                    agent_full_id: agent_full_id_for_stream.clone(),
                                    agent_remote: agent_remote_for_stream.clone(),
                                    error: Some(objectiveai_sdk::error::ResponseError {
                                        code: e.status(),
                                        message: e.message()
                                            .unwrap_or(serde_json::Value::Null),
                                    }),
                                    ..Default::default()
                                },
                            )
                        }
                    });
                    let boxed: Pin<Box<dyn Stream<Item = StreamItem<Self::State>> + Send>> =
                        Box::pin(StreamOnce::new(first).chain(rest));
                    Ok(boxed)
                }
                None => {
                    return Err(super::Error::EmptyStream);
                }
            }
        }
    }

    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&objectiveai_sdk::agent::openrouter::Continuation>,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
        agent_instance_hierarchy: &str,
    ) -> objectiveai_sdk::agent::openrouter::Continuation {
        use objectiveai_sdk::agent::completions::message::Message;
        let rc_len = request_continuation.map_or(0, |rc| rc.messages.len());
        let cont_len = continuation.map_or(0, |c| c.len());
        let mut all_messages = Vec::with_capacity(rc_len + messages.len() + cont_len);
        if let Some(rc) = request_continuation {
            all_messages.extend_from_slice(&rc.messages);
        }
        all_messages.extend_from_slice(messages);
        if let Some(cont) = continuation {
            all_messages.extend(cont.iter().map(|item| match item {
                ContinuationItem::State(assistant) => Message::Assistant(assistant.clone()),
                ContinuationItem::ToolMessage(t) => Message::Tool(t.clone()),
                ContinuationItem::UserMessage(u) => Message::User(u.clone()),
            }));
        }
        objectiveai_sdk::agent::openrouter::Continuation {
            upstream: objectiveai_sdk::agent::openrouter::Upstream::default(),
            agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            messages: all_messages,
            mcp_sessions,
            ws_session_id: None,
        }
    }
}
