use std::pin::Pin;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use tokio::sync::OnceCell;

use super::super::{ContinuationItem, StreamItem, UpstreamClient};
use super::prompt::Prompt;
use super::stdio::{RunParams, Runner, RunnerStream, RunnerUpdate, StdioEndStatus};
use super::McpServerConfig;
use crate::util::StreamOnce;

/// Gemini client for agent completions.
///
/// Owns the Python runner subprocess for the lifetime of the client.
/// The subprocess is spawned **lazily** on the first `create()` call
/// and reused for every subsequent request — see [`Client::runner_handle`].
/// The runner multiplexes N concurrent streams over a single (stdin,
/// stdout, stderr) triple; the in-flight cap is enforced on the Rust
/// side by a `tokio::sync::Semaphore` inside [`Runner`].
///
/// The gemini runner runs the tool-calling loop INTERNALLY (it dials
/// the MCP proxy itself and resolves tool calls before returning the
/// final answer), so `create()` simply relays the runner's events. The
/// API-side orchestrator tool loop is a no-op for gemini turns (the
/// trailing assistant message never carries an unanswered tool call).
///
/// The runner is also STATELESS: there is no server-side session to
/// resume. Continuation works by replaying the full conversation —
/// `create()` builds the runner `messages` from the prior continuation
/// history plus this turn's messages, and `response_continuation`
/// stores the new full history.
#[derive(Clone)]
pub struct Client {
    pub user_agent: String,
    pub enabled: bool,
    pub rate_limit_max_retries: u64,
    pub rate_limit_max_wait_secs: u64,
    /// FIFO concurrency cap on in-flight runner requests, enforced
    /// inside [`Runner`] by a `tokio::sync::Semaphore`.
    pub query_limit: u64,
    /// Used by the prompt builder to download `http(s):` image URLs.
    pub http_client: reqwest::Client,
    binary_path: Arc<OnceCell<String>>,
    /// Lazily-spawned shared runner.
    runner: Arc<OnceCell<Arc<Runner>>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("user_agent", &self.user_agent)
            .field("enabled", &self.enabled)
            .field("rate_limit_max_retries", &self.rate_limit_max_retries)
            .field("rate_limit_max_wait_secs", &self.rate_limit_max_wait_secs)
            .field("query_limit", &self.query_limit)
            .field("runner_initialized", &self.runner.initialized())
            .finish()
    }
}

impl Client {
    pub fn new(
        user_agent: String,
        enabled: bool,
        rate_limit_max_retries: u64,
        rate_limit_max_wait_secs: u64,
        query_limit: u64,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            user_agent,
            enabled,
            rate_limit_max_retries,
            rate_limit_max_wait_secs,
            query_limit,
            http_client,
            binary_path: Arc::new(OnceCell::new()),
            runner: Arc::new(OnceCell::new()),
        }
    }

    /// Extracts the embedded runner binary to a temp directory and
    /// returns its path. Cached after first extraction. Uses a
    /// content-based hash in the directory name so different API
    /// versions get separate binaries and the same version reuses the
    /// cached binary across restarts.
    async fn binary_path(&self) -> Option<&str> {
        let path = self
            .binary_path
            .get_or_init(|| async {
                let binary = super::gemini_binary::GEMINI_RUNNER;

                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                binary.len().hash(&mut hasher);
                binary[..binary.len().min(4096)].hash(&mut hasher);
                binary[binary.len().saturating_sub(4096)..].hash(&mut hasher);
                let hash = hasher.finish();

                let binary_name = if cfg!(windows) {
                    "objectiveai-gemini-sdk-runner.exe"
                } else {
                    "objectiveai-gemini-sdk-runner"
                };

                let dir = std::env::temp_dir()
                    .join(format!("objectiveai-gemini-sdk-runner-{hash:016x}"));
                let path = dir.join(binary_name);

                if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
                    let _ = tokio::fs::create_dir_all(&dir).await;
                    if tokio::fs::write(&path, binary).await.is_err() {
                        return String::new();
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = tokio::fs::set_permissions(
                            &path,
                            std::fs::Permissions::from_mode(0o755),
                        )
                        .await;
                    }
                }

                path.to_string_lossy().to_string()
            })
            .await;
        if path.is_empty() {
            None
        } else {
            Some(path.as_str())
        }
    }

    /// Get-or-init the shared runner subprocess. The first caller to
    /// hit this on a given `Client` pays the spawn cost; subsequent
    /// callers receive a clone of the same `Arc<Runner>`.
    async fn runner_handle(&self) -> Result<Arc<Runner>, super::Error> {
        let query_limit = self.query_limit;
        let binary_path = self
            .binary_path()
            .await
            .ok_or_else(|| {
                super::Error::Spawn(
                    "failed to extract gemini-sdk-runner binary".to_string(),
                )
            })?
            .to_owned();

        let runner = self
            .runner
            .get_or_try_init(|| async move {
                let r = Runner::spawn(&binary_path, query_limit)
                    .await
                    .map_err(|e| super::Error::Spawn(e.to_string()))?;
                Ok::<_, super::Error>(Arc::new(r))
            })
            .await?;
        Ok(runner.clone())
    }
}

/// Build the `mcp_servers` map that goes into [`RunParams`] from the
/// per-agent proxy connection. At most a single entry pointing the
/// runner at the proxy with the agent's pre-initialized
/// `Mcp-Session-Id` header. The wire shape stays a name-keyed map; the
/// name is sourced from the connection's `server_info.name`.
fn build_mcp_servers(
    mcp_connection: Option<&objectiveai_sdk::mcp::Connection>,
) -> IndexMap<String, McpServerConfig> {
    let mut servers = IndexMap::new();
    if let Some(conn) = mcp_connection {
        servers.insert(
            conn.initialize_result.server_info.name.clone(),
            McpServerConfig::from(conn),
        );
    }
    servers
}

/// Validates that the response_format is compatible with Gemini.
///
/// Only `None` or `Text` formats are supported.
fn validate_response_format(
    agent_instance_hierarchy: &str,
    response_format: &Option<objectiveai_sdk::agent::completions::request::ResponseFormatParam>,
) -> Result<(), super::Error> {
    use objectiveai_sdk::agent::completions::request::{
        ResponseFormat, ResponseFormatParam,
    };

    match response_format {
        None => Ok(()),
        Some(ResponseFormatParam::Single(ResponseFormat::Text)) => Ok(()),
        Some(ResponseFormatParam::PerAgent(map)) => match map.get(agent_instance_hierarchy) {
            None => Ok(()),
            Some(ResponseFormat::Text) => Ok(()),
            Some(_) => Err(super::Error::UnsupportedResponseFormat),
        },
        Some(_) => Err(super::Error::UnsupportedResponseFormat),
    }
}

impl
    UpstreamClient<
        objectiveai_sdk::agent::gemini::Agent,
        objectiveai_sdk::agent::gemini::Continuation,
    > for Client
{
    type State = super::State;
    type Stream = Pin<
        Box<dyn Stream<Item = StreamItem<Self::State>> + Send + 'static>,
    >;
    type Error = super::Error;

    #[allow(unused_variables)]
    fn create(
        &self,
        id: &str,
        created: u64,
        agent: &objectiveai_sdk::agent::gemini::Agent,
        request_continuation: Option<&objectiveai_sdk::agent::gemini::Continuation>,
        params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        continuation: Option<&[ContinuationItem<Self::State>]>,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        _tools_enabled: bool,
        agent_instance_hierarchy: &str,
        agent_id: &str,
        agent_full_id: &str,
        agent_remote: Option<&objectiveai_sdk::RemotePath>,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send + 'static
    {
        let enabled = self.enabled;
        let is_byok = byok.is_some();
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

        async move {
            if !enabled {
                return Err(super::Error::NotEnabled);
            }

            if is_byok {
                return Err(super::Error::InvalidByok);
            }

            validate_response_format(&agent.id, &params.response_format)?;

            // Build the full runner conversation: prior continuation
            // history + this turn's messages / continuation items.
            let prompt = Prompt::new(
                &client.http_client,
                &messages,
                continuation.as_deref(),
                request_continuation.as_ref(),
            )
            .await?;

            // assistant_index = sum of message_count for State items,
            // plus 1 per ToolMessage. UserMessage items don't bump the
            // index. Mirrors codex's accounting.
            let assistant_index = continuation
                .as_deref()
                .map(|c| {
                    c.iter()
                        .map(|item| match item {
                            ContinuationItem::State(s) => s.message_count,
                            ContinuationItem::ToolMessage(_) => 1,
                            ContinuationItem::UserMessage(_) => 0,
                        })
                        .sum::<u64>()
                })
                .unwrap_or(0);

            let mcp_servers = build_mcp_servers(mcp_connection.as_ref());

            // Lazy-spawn (or reuse) the runner subprocess.
            let runner = client.runner_handle().await?;

            // Stable session id: reuse the request continuation's id, or
            // mint a fresh one on the first turn. NOT used to resume any
            // server session (the runner is stateless) — purely a stable
            // id carried on the public continuation.
            let session_id = request_continuation
                .as_ref()
                .map(|rc| rc.session_id.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let system_prompt: Option<&str> = if prompt.system_prompt.is_empty() {
                None
            } else {
                Some(prompt.system_prompt.as_str())
            };

            let effort = agent.base.effort.map(|e| e.as_str());
            // The agent's `thinking` defaults to enabled (prepared to
            // `None` when true); forward the bool when explicitly set.
            let thinking = agent.base.thinking;

            let run_params = RunParams {
                model: agent.base.model.as_str(),
                messages: &prompt.messages,
                system_prompt,
                effort,
                thinking,
                web_search_enabled: agent.base.web_search_enabled,
                mcp_servers: &mcp_servers,
                agent_instance_hierarchy: Some(agent_instance_hierarchy.as_str()),
            };

            let rx: RunnerStream = runner
                .create_stream(id.clone(), run_params)
                .await
                .map_err(|e| super::Error::Spawn(e.to_string()))?;

            let id_for_chunks = id.clone();
            let model = agent.base.model.clone();
            // The full conversation we sent — stored verbatim as the
            // base of the new continuation history. The trailing
            // assistant turn (text + tool calls) the runner produced is
            // appended below as events arrive.
            let sent_messages = prompt.messages.clone();
            let session_id_for_state = session_id.clone();
            // Clones for the outer error-mapping closure.
            let agent_instance_hierarchy_for_stream = agent_instance_hierarchy.clone();
            let agent_id_for_stream = agent_id.clone();
            let agent_full_id_for_stream = agent_full_id.clone();
            let agent_remote_for_stream = agent_remote.clone();

            let internal_stream = async_stream::stream! {
                let mut rx = rx;

                let mut had_error = false;
                let mut msg_index = assistant_index;
                // Count assistant messages this turn for State bookkeeping.
                let mut emitted_assistant = 0u64;
                // Accumulate the model turn's text + tool calls so the
                // response continuation history mirrors what the model
                // produced (the runner is stateless and won't remember).
                let mut turn_text = String::new();
                let mut turn_tool_calls: Vec<
                    objectiveai_sdk::agent::gemini::ToolCall,
                > = Vec::new();
                let mut turn_tool_results: Vec<
                    objectiveai_sdk::agent::gemini::Message,
                > = Vec::new();
                let mut saw_any_assistant = false;

                loop {
                    let update = match rx.next().await {
                        Some(u) => u,
                        None => {
                            yield Err(super::Error::NoOutput);
                            had_error = true;
                            break;
                        }
                    };

                    match update {
                        RunnerUpdate::Event(event) => {
                            // Mirror the event into the accumulated turn
                            // history BEFORE mapping (mapping consumes
                            // the event).
                            match &event {
                                super::GeminiEvent::Text { text } => {
                                    turn_text.push_str(text);
                                }
                                super::GeminiEvent::ToolUse { id, name, input } => {
                                    turn_tool_calls.push(
                                        objectiveai_sdk::agent::gemini::ToolCall {
                                            id: id.clone(),
                                            name: name.clone(),
                                            args: input.clone(),
                                        },
                                    );
                                }
                                super::GeminiEvent::ToolResult {
                                    tool_use_id,
                                    content,
                                    is_error,
                                } => {
                                    turn_tool_results.push(
                                        objectiveai_sdk::agent::gemini::Message::Tool {
                                            tool_call_id: tool_use_id.clone(),
                                            name: String::new(),
                                            content: content.clone(),
                                            is_error: *is_error,
                                        },
                                    );
                                }
                                _ => {}
                            }

                            let is_tool_result = matches!(
                                event,
                                super::GeminiEvent::ToolResult { .. }
                            );
                            let is_assistant_content = matches!(
                                event,
                                super::GeminiEvent::Text { .. }
                                    | super::GeminiEvent::Thinking { .. }
                                    | super::GeminiEvent::ToolUse { .. }
                            );

                            if let Some(chunk) = super::stream_event::into_downstream(
                                event,
                                id_for_chunks.clone(),
                                created,
                                model.clone(),
                                msg_index,
                                is_byok,
                                cost_multiplier,
                                objectiveai_sdk::agent::Upstream::Gemini,
                                agent_instance_hierarchy.clone(),
                                agent_id.clone(),
                                agent_full_id.clone(),
                                agent_remote.clone(),
                            ) {
                                if is_assistant_content && !saw_any_assistant {
                                    saw_any_assistant = true;
                                    emitted_assistant += 1;
                                }
                                yield Ok(StreamItem::Chunk(chunk));
                                // A tool_result `Tool` chunk finishes the
                                // current assistant index — advance so the
                                // trailing answer / usage lands on a fresh
                                // index with no tool_calls. (Matches the
                                // orchestrator's `chunk_finishes_assistant`.)
                                if is_tool_result {
                                    msg_index += 1;
                                    saw_any_assistant = false;
                                }
                            }
                        }
                        RunnerUpdate::End(StdioEndStatus::Ok) => break,
                        RunnerUpdate::End(StdioEndStatus::Error { error }) => {
                            yield Err(super::Error::Run(error));
                            had_error = true;
                            break;
                        }
                        RunnerUpdate::Diag { level: _, message: _ } => {
                            // Diags are informational; no downstream
                            // channel for them at this layer.
                        }
                        RunnerUpdate::Fatal(message) => {
                            yield Err(super::Error::Stderr(message));
                            had_error = true;
                            break;
                        }
                        RunnerUpdate::RunnerExited => {
                            yield Err(super::Error::NoOutput);
                            had_error = true;
                            break;
                        }
                    }
                }

                if !had_error {
                    // Build the new full history: everything we sent,
                    // plus this turn's model output (text + tool calls)
                    // and any tool results the runner resolved.
                    let mut new_messages = sent_messages;
                    let model_content =
                        if turn_text.is_empty() {
                            Vec::new()
                        } else {
                            vec![objectiveai_sdk::agent::gemini::ContentPart::Text {
                                text: turn_text,
                            }]
                        };
                    if !model_content.is_empty() || !turn_tool_calls.is_empty() {
                        new_messages.push(
                            objectiveai_sdk::agent::gemini::Message::Model {
                                content: model_content,
                                tool_calls: turn_tool_calls,
                            },
                        );
                    }
                    new_messages.extend(turn_tool_results);

                    let message_count = msg_index
                        .saturating_sub(assistant_index)
                        .max(emitted_assistant)
                        .max(1);
                    yield Ok(StreamItem::State(super::State {
                        session_id: session_id_for_state,
                        message_count,
                        messages: new_messages,
                    }));
                }
            };

            // First-chunk-not-error contract.
            let mut stream = Box::pin(internal_stream);
            match stream.next().await {
                Some(Err(e)) => Err(e),
                Some(Ok(first)) => {
                    let id_for_stream = id.clone();
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
                                        message: e.message().unwrap_or(serde_json::Value::Null),
                                    }),
                                    ..Default::default()
                                },
                            )
                        }
                    });
                    let boxed: Pin<
                        Box<dyn Stream<Item = StreamItem<Self::State>> + Send>,
                    > = Box::pin(StreamOnce::new(first).chain(rest));
                    Ok(boxed)
                }
                None => Err(super::Error::NoOutput),
            }
        }
    }

    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&objectiveai_sdk::agent::gemini::Continuation>,
        _messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
        agent_instance_hierarchy: &str,
    ) -> objectiveai_sdk::agent::gemini::Continuation {
        // The full replay history is carried on the most recent `State`
        // item: `create()` builds it from the prior history + this
        // turn's input messages + the model turn the runner produced
        // (assistant text, tool calls, tool results). Prefer it. If no
        // State was produced (the turn failed before completing), fall
        // back to the prior wire history so we at least preserve the
        // inputs the conversation had so far.
        let messages = continuation
            .and_then(|items| {
                items.iter().rev().find_map(|item| match item {
                    ContinuationItem::State(state) => {
                        Some(state.messages.clone())
                    }
                    _ => None,
                })
            })
            .or_else(|| request_continuation.map(|rc| rc.messages.clone()))
            .unwrap_or_default();

        let session_id = continuation
            .and_then(|items| {
                items.iter().rev().find_map(|item| match item {
                    ContinuationItem::State(state) => {
                        if state.session_id.is_empty() {
                            None
                        } else {
                            Some(state.session_id.clone())
                        }
                    }
                    _ => None,
                })
            })
            .or_else(|| request_continuation.map(|rc| rc.session_id.clone()))
            .filter(|s| !s.is_empty())
            .unwrap_or_default();

        objectiveai_sdk::agent::gemini::Continuation {
            upstream: objectiveai_sdk::agent::gemini::Upstream::default(),
            agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            session_id,
            mcp_sessions,
            messages,
        }
    }
}
