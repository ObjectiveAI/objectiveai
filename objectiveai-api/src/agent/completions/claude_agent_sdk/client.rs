use std::pin::Pin;
use std::sync::Arc;
use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use tokio::sync::OnceCell;

use super::super::{ContinuationItem, StreamItem, UpstreamClient};
use super::mcp_server_config::{McpHttpServerConfig, McpServerConfig};
use super::prompt::Prompt;
use super::sdk_message::SDKMessage;
use super::stdio::{RunParams, Runner, RunnerStream, RunnerUpdate, StdioEndStatus};
use crate::util::StreamOnce;

/// Claude Agent SDK client for agent completions.
///
/// Owns the Python runner subprocess for the lifetime of the
/// client. The subprocess is spawned **lazily** on the first
/// `create()` call and reused for every subsequent request — see
/// [`Client::runner_handle`]. The runner multiplexes N concurrent
/// streams over a single (stdin, stdout, stderr) triple; the in-flight
/// cap is enforced on the Rust side by a `tokio::sync::Semaphore`
/// inside [`Runner`].
#[derive(Clone)]
pub struct Client {
    pub user_agent: String,
    pub enabled: bool,
    pub rate_limit_max_retries: u64,
    pub rate_limit_max_wait_secs: u64,
    /// FIFO concurrency cap on in-flight runner requests, enforced
    /// inside [`Runner`] by a `tokio::sync::Semaphore`. Surplus
    /// requests wait for a permit before their `run` line is sent to
    /// the Python runner subprocess.
    pub query_limit: u64,
    /// Layout root (`OBJECTIVEAI_DIR`); the runner binary is loaded from
    /// `<objectiveai_dir>/bin/objectiveai-claude-agent-sdk-runner[.exe]`.
    objectiveai_dir: std::path::PathBuf,
    binary_path: Arc<OnceCell<String>>,
    /// Lazily-spawned shared runner. Initialized on first request via
    /// `tokio::sync::OnceCell::get_or_try_init`. All concurrent
    /// `create()` callers race for the same singleton; only one
    /// initializer runs.
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
        objectiveai_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            user_agent,
            enabled,
            rate_limit_max_retries,
            rate_limit_max_wait_secs,
            query_limit,
            objectiveai_dir,
            binary_path: Arc::new(OnceCell::new()),
            runner: Arc::new(OnceCell::new()),
        }
    }

    /// Path to the SDK runner binary in `<OBJECTIVEAI_DIR>/bin/`.
    ///
    /// The runner is shipped alongside the other objectiveai binaries
    /// (no longer embedded into the api). Computed once and cached in a
    /// `tokio::sync::OnceCell`. If the binary is missing, `Runner::spawn`
    /// surfaces a clear spawn error downstream.
    async fn binary_path(&self) -> Option<&str> {
        let path = self
            .binary_path
            .get_or_init(|| async {
                let binary_name = if cfg!(windows) {
                    "objectiveai-claude-agent-sdk-runner.exe"
                } else {
                    "objectiveai-claude-agent-sdk-runner"
                };
                self.objectiveai_dir
                    .join("bin")
                    .join(binary_name)
                    .to_string_lossy()
                    .to_string()
            })
            .await;
        Some(path.as_str())
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
                    "failed to extract claude-agent-sdk-runner binary".to_string(),
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

/// Build the `mcp_servers` map that goes into [`RunParams`]. With the
/// per-agent proxy connection, this is at most a single entry pointing
/// the SDK's child at the proxy with the agent's pre-initialized
/// `Mcp-Session-Id` header so it resumes the parent's session rather
/// than re-issuing `initialize`. Header construction is delegated to
/// `McpHttpServerConfig::from(&Connection)` to keep the merge with
/// `conn.headers` (User-Agent, Authorization, custom X-*) in one place.
fn build_mcp_servers(
    mcp_connection: Option<&objectiveai_sdk::mcp::Connection>,
) -> IndexMap<String, McpServerConfig> {
    let mut servers = IndexMap::new();
    if let Some(conn) = mcp_connection {
        servers.insert(
            conn.initialize_result.server_info.name.clone(),
            McpServerConfig::Http(McpHttpServerConfig::from(conn)),
        );
    }
    servers
}

/// Validates that the response_format is compatible with the Claude Agent SDK.
///
/// Only `None` or `Text` formats are supported.
fn validate_response_format(
    agent_instance_hierarchy: &str,
    response_format: &Option<objectiveai_sdk::agent::completions::request::ResponseFormatParam>,
) -> Result<(), super::Error> {
    use objectiveai_sdk::agent::completions::request::{ResponseFormat, ResponseFormatParam};

    match response_format {
        None => Ok(()),
        Some(ResponseFormatParam::Single(ResponseFormat::Text)) => Ok(()),
        Some(ResponseFormatParam::PerAgent(map)) => {
            match map.get(agent_instance_hierarchy) {
                None => Ok(()),
                Some(ResponseFormat::Text) => Ok(()),
                Some(_) => Err(super::Error::UnsupportedResponseFormat),
            }
        }
        Some(_) => Err(super::Error::UnsupportedResponseFormat),
    }
}

impl UpstreamClient<objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation> for Client {
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
        agent: &objectiveai_sdk::agent::claude_agent_sdk::Agent,
        request_continuation: Option<&objectiveai_sdk::agent::claude_agent_sdk::Continuation>,
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
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static {
        let enabled = self.enabled;
        let tools_enabled = _tools_enabled;
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

            // Build prompt from messages + continuation (handles continuation validation).
            let prompt = Prompt::new(&messages, continuation.as_deref(), request_continuation.as_ref())?;

            // When tools are disabled for this iteration, give the SDK
            // an empty MCP server map so it never tries to connect.
            let mcp_servers = if tools_enabled {
                build_mcp_servers(mcp_connection.as_ref())
            } else {
                IndexMap::new()
            };

            // Compute assistant_index from continuation. State items
            // carry a message_count (may be >1 since the SDK handles
            // its own multi-turn loop). Other items count as 1.
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

            // Lazy-spawn (or reuse) the runner subprocess.
            let runner = client.runner_handle().await?;

            // Build the params object — borrows from locals in this
            // async block, valid for the duration of the await on
            // create_stream.
            let session_id = prompt.message.session_id.as_str();
            let resume_arg: Option<&str> =
                if session_id.is_empty() { None } else { Some(session_id) };
            let user_agent_arg: Option<&str> =
                if client.user_agent.is_empty() { None } else { Some(client.user_agent.as_str()) };

            let run_params = RunParams {
                model: agent.base.model.as_str(),
                message: &prompt.message,
                system_prompt: prompt.system_prompt.as_deref(),
                effort: agent.base.effort,
                thinking_disabled: agent.base.thinking == Some(false),
                mcp_servers: &mcp_servers,
                resume: resume_arg,
                user_agent: user_agent_arg,
                agent_instance_hierarchy: Some(agent_instance_hierarchy.as_str()),
                rate_limit_max_retries: client.rate_limit_max_retries,
                rate_limit_max_wait_secs: client.rate_limit_max_wait_secs,
            };

            // Each agent-completions request gets its own caller-side
            // id. We use `id` (the upstream id) rather than minting a
            // separate UUID — the upstream id is already unique per
            // request and lets the runner's diag lines be cross-
            // referenced against agent-completion logs. The returned
            // RunnerStream auto-cancels on drop unless it saw a
            // terminal update.
            let mut rx = runner
                .create_stream(id.clone(), run_params)
                .await
                .map_err(|e| super::Error::Spawn(e.to_string()))?;

            let id_for_chunks = id.clone();
            let agent_instance_hierarchy = agent.id.clone();
            // Clones for the outer error-mapping closure, taken before
            // `internal_stream` moves the originals into its generator.
            let agent_instance_hierarchy_for_stream = agent_instance_hierarchy.clone();
            let agent_id_for_stream = agent_id.clone();
            let agent_full_id_for_stream = agent_full_id.clone();
            let agent_remote_for_stream = agent_remote.clone();

            let internal_stream = async_stream::stream! {
                // RunnerStream's Drop handles cancellation automatically.
                let mut rx = rx;

                let mut latest_session_id = String::new();
                let mut had_error = false;
                let mut msg_index = assistant_index;
                // Most-recent assistant index, so the SDK's trailing
                // ResultMessage (a usage/cost summary, not a real
                // second turn) can re-use it. Per protocol, assistant
                // messages never sit back-to-back at distinct indices
                // — they alternate with tool messages — so the trailer
                // must merge into the assistant that just finished.
                let mut last_assistant_index: Option<u64> = None;

                loop {
                    let update = match rx.next().await {
                        Some(u) => u,
                        None => {
                            // The RunnerStream closed without sending
                            // an end (already-terminal updates close
                            // it cleanly via marking it complete first
                            // — getting None here means the runner
                            // died mid-flight).
                            yield Err(super::Error::NoOutput);
                            had_error = true;
                            break;
                        }
                    };

                    match update {
                        RunnerUpdate::Event(sdk_msg) => {
                            // Track latest session_id.
                            if let Some(sid) = sdk_msg.session_id() {
                                if !sid.is_empty() {
                                    latest_session_id = sid.to_string();
                                }
                            }

                            // ResultMessage merges into the last
                            // assistant index instead of advancing.
                            let effective_index = match &sdk_msg {
                                SDKMessage::ResultMessage(_) => {
                                    last_assistant_index.unwrap_or(msg_index)
                                }
                                _ => msg_index,
                            };

                            match sdk_msg.into_downstream(
                                id_for_chunks.clone(),
                                created,
                                effective_index,
                                is_byok,
                                cost_multiplier,
                                objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
                                agent_instance_hierarchy.clone(),
                                agent_id.clone(),
                                agent_full_id.clone(),
                                agent_remote.clone(),
                            ) {
                                Some(Ok(chunk)) => {
                                    use objectiveai_sdk::agent::completions::response::streaming::MessageChunk;
                                    let mut advances_index = false;
                                    for m in &chunk.messages {
                                        match m {
                                            MessageChunk::Assistant(a) => {
                                                last_assistant_index = Some(a.index);
                                                if a.finish_reason.is_some() {
                                                    advances_index = true;
                                                }
                                            }
                                            MessageChunk::Tool(_) => {
                                                advances_index = true;
                                            }
                                        }
                                    }
                                    yield Ok(StreamItem::Chunk(chunk));
                                    if advances_index {
                                        msg_index += 1;
                                    }
                                }
                                Some(Err(e)) => {
                                    yield Err(e);
                                    had_error = true;
                                    break;
                                }
                                None => {
                                    // Ignored message type.
                                }
                            }
                        }
                        // Terminal updates: RunnerStream marks itself
                        // complete on these.
                        RunnerUpdate::End(StdioEndStatus::Ok) => break,
                        RunnerUpdate::End(StdioEndStatus::Error { error }) => {
                            yield Err(super::Error::Stderr(error));
                            had_error = true;
                            break;
                        }
                        RunnerUpdate::Diag { level: _, message: _ } => {
                            // Diags are informational (rate-limit
                            // retries etc.) — no downstream channel
                            // for them at this layer. Drop them; the
                            // user-visible signal is the eventual
                            // event/end.
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
                    yield Ok(StreamItem::State(super::State {
                        session_id: latest_session_id,
                        message_count: msg_index - assistant_index,
                    }));
                }
            };

            // Await the first stream item. If it is an error,
            // return Err so the caller never sees an error as the
            // first yielded item (per the upstream contract).
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
                None => Err(super::Error::NoOutput),
            }
        }
    }

    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&objectiveai_sdk::agent::claude_agent_sdk::Continuation>,
        _messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
        agent_instance_hierarchy: &str,
    ) -> objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        // Extract session_id from last State in continuation, fall back to request continuation.
        let session_id = continuation
            .and_then(|items| {
                items.iter().rev().find_map(|item| match item {
                    ContinuationItem::State(state) => {
                        if state.session_id.is_empty() { None } else { Some(state.session_id.clone()) }
                    }
                    _ => None,
                })
            })
            .or_else(|| request_continuation.map(|rc| rc.session_id.clone()))
            .unwrap_or_default();

        objectiveai_sdk::agent::claude_agent_sdk::Continuation {
            upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::default(),
            agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            session_id,
            mcp_sessions,
        }
    }
}
