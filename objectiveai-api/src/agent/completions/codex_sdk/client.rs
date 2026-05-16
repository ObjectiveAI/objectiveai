use std::pin::Pin;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use tokio::sync::OnceCell;

use super::super::{ContinuationItem, StreamItem, UpstreamClient};
use super::prompt::Prompt;
use super::stdio::{RunParams, Runner, RunnerStream, RunnerUpdate, StdioEndStatus};
use super::{McpServerConfig, ModelReasoningEffort};
use crate::util::StreamOnce;

/// Codex SDK client for agent completions.
///
/// Owns the Python runner subprocess for the lifetime of the client.
/// The subprocess is spawned **lazily** on the first `create()` call
/// and reused for every subsequent request — see [`Client::runner_handle`].
/// The runner multiplexes N concurrent streams over a single (stdin,
/// stdout, stderr) triple; the in-flight cap is enforced on the Rust
/// side by a `tokio::sync::Semaphore` inside [`Runner`].
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
    /// Used by the prompt builder to download `http(s):` image URLs.
    pub http_client: reqwest::Client,
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
    /// returns its path. Cached after first extraction so the expensive
    /// write happens only once even under concurrent first-callers.
    /// Uses a content-based hash in the directory name so different API
    /// versions get separate binaries and the same version reuses the
    /// cached binary across restarts.
    ///
    /// Returns `None` only if the on-disk extraction of the embedded
    /// runner binary failed (e.g. temp dir not writable). In normal
    /// operation this returns `Some(path)`.
    async fn binary_path(&self) -> Option<&str> {
        let path = self
            .binary_path
            .get_or_init(|| async {
                let binary = super::codex_sdk_binary::CODEX_SDK_RUNNER;

                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                binary.len().hash(&mut hasher);
                binary[..binary.len().min(4096)].hash(&mut hasher);
                binary[binary.len().saturating_sub(4096)..].hash(&mut hasher);
                let hash = hasher.finish();

                let binary_name = if cfg!(windows) {
                    "objectiveai-codex-sdk-runner.exe"
                } else {
                    "objectiveai-codex-sdk-runner"
                };

                let dir = std::env::temp_dir()
                    .join(format!("objectiveai-codex-sdk-runner-{hash:016x}"));
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
                    "failed to extract codex-sdk-runner binary".to_string(),
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
/// `Mcp-Session-Id` header. The wire shape stays a name-keyed map (the
/// Python runner expects one) but the name is sourced from the
/// connection's `server_info.name` rather than hardcoded here.
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

/// Validates that the response_format is compatible with the Codex SDK.
///
/// Only `None` or `Text` formats are supported.
fn validate_response_format(
    agent_id: &str,
    response_format: &Option<objectiveai_sdk::agent::completions::request::ResponseFormatParam>,
) -> Result<(), super::Error> {
    use objectiveai_sdk::agent::completions::request::{
        ResponseFormat, ResponseFormatParam,
    };

    match response_format {
        None => Ok(()),
        Some(ResponseFormatParam::Single(ResponseFormat::Text)) => Ok(()),
        Some(ResponseFormatParam::PerAgent(map)) => match map.get(agent_id) {
            None => Ok(()),
            Some(ResponseFormat::Text) => Ok(()),
            Some(_) => Err(super::Error::UnsupportedResponseFormat),
        },
        Some(_) => Err(super::Error::UnsupportedResponseFormat),
    }
}

/// Map an `objectiveai_sdk::agent::codex_sdk::Effort` to the runner's
/// `ModelReasoningEffort` enum.
fn into_codex_effort(
    effort: objectiveai_sdk::agent::codex_sdk::Effort,
) -> ModelReasoningEffort {
    use objectiveai_sdk::agent::codex_sdk::Effort;
    match effort {
        Effort::Minimal => ModelReasoningEffort::Minimal,
        Effort::Low => ModelReasoningEffort::Low,
        Effort::Medium => ModelReasoningEffort::Medium,
        Effort::High => ModelReasoningEffort::High,
    }
}

impl
    UpstreamClient<
        objectiveai_sdk::agent::codex_sdk::Agent,
        objectiveai_sdk::agent::codex_sdk::Continuation,
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
        agent: &objectiveai_sdk::agent::codex_sdk::Agent,
        request_continuation: Option<&objectiveai_sdk::agent::codex_sdk::Continuation>,
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

        async move {
            if !enabled {
                return Err(super::Error::NotEnabled);
            }

            if is_byok {
                return Err(super::Error::InvalidByok);
            }

            validate_response_format(&agent.id, &params.response_format)?;

            // Per-request CWD tempdir, named with the request id so
            // ops can correlate `<temp_dir>/<id>` against logs. The
            // `TempDir` is owned by the streaming task below and
            // dropped (recursively deleted) when the stream ends.
            let cwd = tempfile::Builder::new()
                .prefix(id.as_str())
                .rand_bytes(0)
                .tempdir()
                .map_err(|e| super::Error::Io(e.to_string()))?;

            let prompt = Prompt::new(
                cwd.path(),
                &client.http_client,
                &messages,
                continuation.as_deref(),
                request_continuation.as_ref(),
            )
            .await?;

            // assistant_index = sum of message_count for State items,
            // plus 1 per ToolMessage. UserMessage items don't bump the
            // index (they're just additional input content).
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

            let resume_arg: Option<&str> = if prompt.thread_id.is_empty() {
                None
            } else {
                Some(prompt.thread_id.as_str())
            };

            let cwd_str = cwd.path().to_string_lossy().to_string();
            let run_params = RunParams {
                model: agent.base.model.as_str(),
                input: &prompt.input,
                cwd: &cwd_str,
                effort: agent.base.effort.map(into_codex_effort),
                web_search_enabled: agent.base.web_search_enabled,
                resume: resume_arg,
                mcp_servers: &mcp_servers,
            };

            // Each agent-completions request gets its own caller-side
            // id. We use `id` (the upstream id) rather than minting a
            // separate UUID — the upstream id is already unique per
            // request and matches the cwd directory name. The returned
            // RunnerStream auto-cancels on drop unless it saw a
            // terminal update.
            let rx: RunnerStream = runner
                .create_stream(id.clone(), run_params)
                .await
                .map_err(|e| super::Error::Spawn(e.to_string()))?;

            let id_for_chunks = id.clone();
            let agent_id = agent.id.clone();
            let model = agent.base.model.clone();
            let initial_thread_id = prompt.thread_id.clone();

            let internal_stream = async_stream::stream! {
                // Move the cwd TempDir into the stream task so it
                // outlives the runner request. Dropped (recursively
                // deleted) when this generator finishes — success,
                // error, or consumer drop.
                let _cwd = cwd;
                let mut rx = rx;

                let mut latest_thread_id = initial_thread_id;
                let mut had_error = false;
                let mut msg_index = assistant_index;
                let mut emitted_assistant = 0u64;

                loop {
                    let update = match rx.next().await {
                        Some(u) => u,
                        None => {
                            // RunnerStream closed without sending an
                            // end (already-terminal updates close it
                            // cleanly via marking it complete first
                            // — getting None here means the runner
                            // died mid-flight).
                            yield Err(super::Error::NoOutput);
                            had_error = true;
                            break;
                        }
                    };

                    match update {
                        RunnerUpdate::Event(event) => {
                            // Track the latest thread_id from
                            // `thread.started` events before mapping.
                            if let super::ThreadEvent::Known(
                                super::KnownThreadEvent::ThreadStarted(ref ts)
                            ) = event {
                                if !ts.thread_id.is_empty() {
                                    latest_thread_id = ts.thread_id.clone();
                                }
                            }

                            match super::stream_event::into_downstream(
                                event,
                                id_for_chunks.clone(),
                                created,
                                agent_id.clone(),
                                model.clone(),
                                msg_index,
                                is_byok,
                                cost_multiplier,
                                objectiveai_sdk::agent::Upstream::CodexSdk,
                                &latest_thread_id,
                            ) {
                                Some(Ok(chunk)) => {
                                    let advances = chunk_finishes_assistant(&chunk);
                                    if chunk_has_assistant(&chunk) {
                                        emitted_assistant += 1;
                                    }
                                    yield Ok(StreamItem::Chunk(chunk));
                                    if advances {
                                        msg_index += 1;
                                    }
                                }
                                Some(Err(e)) => {
                                    yield Err(e);
                                    had_error = true;
                                    break;
                                }
                                None => {}
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
                    let message_count = msg_index.saturating_sub(assistant_index)
                        .max(emitted_assistant);
                    yield Ok(StreamItem::State(super::State {
                        thread_id: latest_thread_id,
                        message_count,
                    }));
                }
            };

            // First-chunk-not-error contract: await the first item
            // and surface it as Err if it's an error.
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
        request_continuation: Option<&objectiveai_sdk::agent::codex_sdk::Continuation>,
        _messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
    ) -> objectiveai_sdk::agent::codex_sdk::Continuation {
        let thread_id = continuation
            .and_then(|items| {
                items.iter().rev().find_map(|item| match item {
                    ContinuationItem::State(state) => {
                        if state.thread_id.is_empty() {
                            None
                        } else {
                            Some(state.thread_id.clone())
                        }
                    }
                    _ => None,
                })
            })
            .or_else(|| request_continuation.map(|rc| rc.thread_id.clone()))
            .unwrap_or_default();

        objectiveai_sdk::agent::codex_sdk::Continuation {
            upstream: objectiveai_sdk::agent::codex_sdk::Upstream::default(),
            thread_id,
            mcp_sessions,
        }
    }
}

fn chunk_has_assistant(
    chunk: &objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
) -> bool {
    use objectiveai_sdk::agent::completions::response::streaming::MessageChunk;
    chunk
        .messages
        .iter()
        .any(|m| matches!(m, MessageChunk::Assistant(_)))
}

fn chunk_finishes_assistant(
    chunk: &objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
) -> bool {
    use objectiveai_sdk::agent::completions::response::streaming::MessageChunk;
    chunk.messages.iter().any(|m| match m {
        MessageChunk::Assistant(a) => a.finish_reason.is_some(),
        MessageChunk::Tool(_) => true,
    })
}
