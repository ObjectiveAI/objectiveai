//! Script upstream client — a thin wrapper around the client-side
//! python runtime.
//!
//! The whole "upstream" is one reverse-channel RPC: assemble the FULL
//! conversation (request continuation ++ messages ++ internal
//! continuation items, exactly like mock), ship it with the agent's
//! script definition as [`server_request::Payload::Script`], and let
//! the CLI daemon run the code on its embedded runtime (the same one
//! the `python` command uses). The reply's assistant/tool-only output
//! messages become the agent's SOLE yielded chunk; usage carries no
//! token counts — only the measured wall-clock duration (plus its
//! duration charge).

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AssistantResponseChunk, MessageChunk,
};

use super::super::{ContinuationItem, StreamItem};

type Message = objectiveai_sdk::agent::completions::message::Message;

/// Upstream client for Script agents. Stateless — the per-request
/// reverse-attach handle arrives per `create` call.
pub struct Client;

impl
    super::super::UpstreamClient<
        objectiveai_sdk::agent::script::Agent,
        objectiveai_sdk::agent::script::Continuation,
    > for Client
{
    /// One turn's output as conversation messages — a script may
    /// append several (assistant/tool) messages per turn, so the
    /// state is the whole slice rather than a single assistant
    /// message.
    type State = Vec<Message>;
    type Stream = Pin<Box<dyn Stream<Item = StreamItem<Self::State>> + Send>>;
    type Error = super::Error;

    fn create(
        &self,
        id: &str,
        created: u64,
        agent: &objectiveai_sdk::agent::script::Agent,
        request_continuation: Option<&objectiveai_sdk::agent::script::Continuation>,
        _params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        messages: &[Message],
        _mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        reverse_attach: Option<std::sync::Arc<crate::objectiveai_mcp::ReverseAttachHandle>>,
        continuation: Option<&[ContinuationItem<Self::State>]>,
        _byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        duration_cost: rust_decimal::Decimal,
        _tools_enabled: bool,
        agent_instance_hierarchy: &str,
        agent_id_arg: &str,
        agent_full_id: &str,
        agent_remote: Option<&objectiveai_sdk::RemotePath>,
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static {
        let id = id.to_string();
        let agent_instance_hierarchy = agent_instance_hierarchy.to_string();
        let agent_id_for_chunks = agent_id_arg.to_string();
        let agent_full_id = agent_full_id.to_string();
        let agent_remote = agent_remote.cloned();
        let script = agent.base.script.clone();

        // Build the full conversation: request_continuation -> messages
        // -> continuation (State items splice their whole turn).
        let rc_len = request_continuation.map_or(0, |rc| rc.messages.len());
        let cont_len = continuation.map_or(0, |c| c.len());
        let mut all_messages: Vec<Message> =
            Vec::with_capacity(rc_len + messages.len() + cont_len);
        if let Some(rc) = request_continuation {
            all_messages.extend_from_slice(&rc.messages);
        }
        all_messages.extend_from_slice(messages);
        if let Some(cont) = continuation {
            for item in cont {
                match item {
                    ContinuationItem::State(turn) => {
                        all_messages.extend_from_slice(turn);
                    }
                    ContinuationItem::ToolMessage(t) => {
                        all_messages.push(Message::Tool(t.clone()));
                    }
                    ContinuationItem::UserMessage(u) => {
                        all_messages.push(Message::User(u.clone()));
                    }
                }
            }
        }
        // Message-chunk index base: how many conversation positions the
        // internal continuation already produced (same counting rule as
        // mock's assistant_index).
        let index_base = continuation
            .map(|c| {
                c.iter()
                    .filter(|item| {
                        matches!(
                            item,
                            ContinuationItem::State(_) | ContinuationItem::ToolMessage(_)
                        )
                    })
                    .count() as u64
            })
            .unwrap_or(0);
        // `byok` is meaningless for scripts; carried onto the usage
        // trailer as `is_byok: false` via the flag below.
        let is_byok = false;

        async move {
            let Some(handle) = reverse_attach else {
                return Err(super::Error::ReverseChannelUnavailable);
            };

            use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};

            let start = std::time::Instant::now();
            let rc = handle.channel();
            let request = server_request::Request {
                id: uuid::Uuid::new_v4().to_string(),
                headers: indexmap::IndexMap::new(),
                payload: server_request::Payload::Script(server_request::ScriptRequest {
                    script,
                    messages: all_messages,
                    // TYPED identity (the non-MCP convention): the CLI
                    // applies these to the script's execution context,
                    // so anything the script runs via
                    // `objectiveai.execute` uses THIS agent's identity
                    // (its response id, agent full id, ...). The
                    // sibling response-id group isn't threaded into
                    // upstream clients — `None`.
                    agent_instance_hierarchy: agent_instance_hierarchy.clone(),
                    agent_id: agent_id_for_chunks.clone(),
                    agent_full_id: agent_full_id.clone(),
                    agent_remote: agent_remote
                        .as_ref()
                        .map(|r| serde_json::to_string(r).unwrap()),
                    response_id: id.clone(),
                    response_ids: None,
                }),
            };
            // No timeout — like every other API-direct reverse-channel
            // RPC, the await resolves when the reply arrives or errors
            // when the WS drops (the recv loop's teardown drops the
            // pending oneshot).
            let rx = crate::objectiveai_mcp::send_server_request(&rc.sink, &rc.pending, request)
                .await
                .map_err(|()| super::Error::ChannelClosed)?;
            let response = rx.await.map_err(|_| super::Error::ChannelDropped)?;
            let output = match response.payload {
                server_response::Payload::Script(server_response::JsonRpcResult::Ok {
                    result,
                }) => result.messages,
                server_response::Payload::Script(server_response::JsonRpcResult::Err {
                    code,
                    message,
                    ..
                }) => return Err(super::Error::Script { code, message }),
                _ => return Err(super::Error::WrongVariant),
            };
            if output.is_empty() {
                return Err(super::Error::EmptyOutput);
            }
            let elapsed_ms = start.elapsed().as_millis() as u64;

            // Duration is the script agent's WHOLE usage: no token
            // counts, just the measured wall time and its charge —
            // added raw to both `cost` and `total_cost` (see
            // `crate::duration`).
            let duration_charge = crate::duration::duration_charge(elapsed_ms, duration_cost);
            let mut upstream_duration_ms =
                objectiveai_sdk::agent::completions::response::UpstreamDurationMs::default();
            upstream_duration_ms.script = Some(elapsed_ms);
            let usage = objectiveai_sdk::agent::completions::response::UpstreamUsage {
                completion_tokens: 0,
                prompt_tokens: 0,
                total_tokens: 0,
                completion_tokens_details: None,
                prompt_tokens_details: None,
                cost: duration_charge,
                cost_details: None,
                total_cost: duration_charge,
                upstream_duration_ms,
                cost_multiplier,
                is_byok,
            };
            let mut usage = Some(usage);

            // The turn's state: the output as conversation messages.
            let state: Vec<Message> =
                output.iter().cloned().map(Message::from).collect();

            // The SOLE chunk: every output message as its own
            // MessageChunk. The LAST assistant carries the usage
            // trailer; if the output has no assistant at all, an empty
            // usage-only assistant chunk is appended (the mock trailer
            // pattern) so the run loop still folds the usage.
            let last_assistant = output
                .iter()
                .rposition(|m| {
                    matches!(
                        m,
                        objectiveai_sdk::agent::script::OutputMessage::Assistant(_)
                    )
                });
            let mut message_chunks: Vec<MessageChunk> = Vec::with_capacity(output.len() + 1);
            for (i, message) in output.into_iter().enumerate() {
                let index = index_base + i as u64;
                match message {
                    objectiveai_sdk::agent::script::OutputMessage::Assistant(asst) => {
                        let tool_calls = asst.tool_calls.map(|tcs| {
                            tcs.into_iter()
                                .enumerate()
                                .map(|(k, tc)| {
                                    let objectiveai_sdk::agent::completions::message::AssistantToolCall::Function { id, function } = tc;
                                    objectiveai_sdk::agent::completions::message::AssistantToolCallDelta {
                                        index: k as u64,
                                        r#type: Some(objectiveai_sdk::agent::completions::message::AssistantToolCallType::Function),
                                        id: Some(id),
                                        function: Some(objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                            name: Some(function.name),
                                            arguments: Some(function.arguments),
                                        }),
                                    }
                                })
                                .collect::<Vec<_>>()
                        });
                        let finish_reason = if tool_calls.is_some() {
                            objectiveai_sdk::agent::completions::response::FinishReason::ToolCalls
                        } else {
                            objectiveai_sdk::agent::completions::response::FinishReason::Stop
                        };
                        let chunk_usage = if Some(i) == last_assistant {
                            usage.take()
                        } else {
                            None
                        };
                        message_chunks.push(MessageChunk::Assistant(AssistantResponseChunk {
                            index,
                            created,
                            model: "script".into(),
                            upstream_id: id.clone(),
                            reasoning: asst.reasoning,
                            tool_calls,
                            content: asst.content,
                            refusal: asst.refusal,
                            finish_reason: Some(finish_reason),
                            usage: chunk_usage,
                            ..Default::default()
                        }));
                    }
                    objectiveai_sdk::agent::script::OutputMessage::Tool(tool) => {
                        message_chunks.push(MessageChunk::Tool(
                            objectiveai_sdk::agent::completions::response::ToolResponse {
                                role: Default::default(),
                                index,
                                inner: tool,
                                request_message_ids: None,
                            },
                        ));
                    }
                }
            }
            if let Some(usage) = usage.take() {
                // No assistant message in the output — append the
                // usage-only trailer chunk.
                message_chunks.push(MessageChunk::Assistant(AssistantResponseChunk {
                    index: index_base + message_chunks.len() as u64,
                    created,
                    model: "script".into(),
                    upstream_id: id.clone(),
                    usage: Some(usage),
                    ..Default::default()
                }));
            }

            let chunk = AgentCompletionChunk {
                id: id.clone(),
                agent_instance_hierarchy: agent_instance_hierarchy.clone(),
                agent_id: agent_id_for_chunks.clone(),
                agent_full_id: agent_full_id.clone(),
                agent_remote: agent_remote.clone(),
                created,
                messages: message_chunks,
                object: Default::default(),
                usage: None,
                upstream: objectiveai_sdk::agent::Upstream::Script,
                error: None,
                continuation: None,
                messages_queued: None,
            };

            let stream = async_stream::stream! {
                yield StreamItem::Chunk(chunk);
                yield StreamItem::State(state);
            };
            let boxed: Pin<Box<dyn Stream<Item = StreamItem<Self::State>> + Send>> =
                Box::pin(stream);
            Ok(boxed)
        }
    }

    fn response_continuation(
        &self,
        request_continuation: Option<&objectiveai_sdk::agent::script::Continuation>,
        messages: &[Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
        agent_instance_hierarchy: &str,
    ) -> objectiveai_sdk::agent::script::Continuation {
        let rc_len = request_continuation.map_or(0, |rc| rc.messages.len());
        let cont_len = continuation.map_or(0, |c| c.len());
        let mut all_messages = Vec::with_capacity(rc_len + messages.len() + cont_len);
        if let Some(rc) = request_continuation {
            all_messages.extend_from_slice(&rc.messages);
        }
        all_messages.extend_from_slice(messages);
        if let Some(cont) = continuation {
            for item in cont {
                match item {
                    ContinuationItem::State(turn) => {
                        all_messages.extend_from_slice(turn);
                    }
                    ContinuationItem::ToolMessage(t) => {
                        all_messages.push(Message::Tool(t.clone()));
                    }
                    ContinuationItem::UserMessage(u) => {
                        all_messages.push(Message::User(u.clone()));
                    }
                }
            }
        }
        objectiveai_sdk::agent::script::Continuation {
            upstream: objectiveai_sdk::agent::script::Upstream::default(),
            agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            messages: all_messages,
        }
    }
}
