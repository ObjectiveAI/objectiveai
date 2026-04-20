use std::pin::Pin;
use std::sync::Arc;
use futures::{Stream, StreamExt};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio_stream::wrappers::LinesStream;

use super::super::{ContinuationItem, StreamItem, UpstreamClient};
use super::invention_server::InventionServer;
use super::mcp_server_config::McpHttpServerConfig;
use super::prompt::Prompt;
use super::sdk_message::SDKMessage;
use crate::util::StreamOnce;

/// Claude Agent SDK client for agent completions.
///
/// Extracts the embedded Claude Agent SDK runner binary
/// on first use and spawns it as a subprocess for each query.
#[derive(Debug, Clone)]
pub struct Client {
    pub user_agent: String,
    pub enabled: bool,
    pub rate_limit_max_retries: u64,
    binary_path: Arc<std::sync::OnceLock<String>>,
}

impl Client {
    pub fn new(user_agent: String, enabled: bool, rate_limit_max_retries: u64) -> Self {
        Self {
            user_agent,
            enabled,
            rate_limit_max_retries,
            binary_path: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Extracts the embedded runner binary to a temp directory and returns its path.
    ///
    /// Cached after first extraction. Uses a content-based hash in the directory name
    /// so different API versions get separate binaries and the same version reuses
    /// the cached binary across restarts.
    fn binary_path(&self) -> Option<&str> {
        let path = self.binary_path.get_or_init(|| {
            let binary = super::claude_agent_sdk_binary::CLAUDE_AGENT_SDK_RUNNER;

            // Fast fingerprint: hash length + head/tail for cache key.
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            binary.len().hash(&mut hasher);
            binary[..binary.len().min(4096)].hash(&mut hasher);
            binary[binary.len().saturating_sub(4096)..].hash(&mut hasher);
            let hash = hasher.finish();

            let binary_name = if cfg!(windows) {
                "objectiveai-claude-agent-sdk-runner.exe"
            } else {
                "objectiveai-claude-agent-sdk-runner"
            };

            let dir = std::env::temp_dir()
                .join(format!("objectiveai-sdk-runner-{hash:016x}"));
            let path = dir.join(binary_name);

            if !path.exists() {
                std::fs::create_dir_all(&dir).ok();
                if std::fs::write(&path, binary).is_err() {
                    return String::new();
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &path,
                        std::fs::Permissions::from_mode(0o755),
                    );
                }
            }

            path.to_string_lossy().to_string()
        });
        if path.is_empty() { None } else { Some(path.as_str()) }
    }
}

/// Builds the MCP servers JSON object from connections and an optional invention server.
fn build_mcp_servers_json(
    mcp_connections: &[Arc<crate::mcp::Connection>],
    invention_server: Option<&InventionServer>,
) -> String {
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for conn in mcp_connections {
        let name = &conn.initialize_result.server_info.name;
        *name_counts.entry(name.clone()).or_default() += 1;
    }

    let mut servers: IndexMap<String, serde_json::Value> = IndexMap::new();

    for conn in mcp_connections {
        let name = &conn.initialize_result.server_info.name;
        let key = if name_counts.get(name).copied().unwrap_or(0) > 1 {
            format!("{name} ({})", conn.url)
        } else {
            name.clone()
        };
        let config = McpHttpServerConfig::from(conn.as_ref());
        servers.insert(key, serde_json::to_value(&config).unwrap());
    }

    if let Some(inv) = invention_server {
        let config = inv.mcp_server_config();
        servers.insert(
            "objectiveai-invention".to_string(),
            serde_json::to_value(&config).unwrap(),
        );
    }

    serde_json::to_string(&servers).unwrap()
}

/// Validates that the response_format is compatible with the Claude Agent SDK.
///
/// Only `None` or `Text` formats are supported.
fn validate_response_format(
    agent_id: &str,
    response_format: &Option<objectiveai::agent::completions::request::ResponseFormatParam>,
) -> Result<(), super::Error> {
    use objectiveai::agent::completions::request::{ResponseFormat, ResponseFormatParam};

    match response_format {
        None => Ok(()),
        Some(ResponseFormatParam::Single(ResponseFormat::Text)) => Ok(()),
        Some(ResponseFormatParam::PerAgent(map)) => {
            match map.get(agent_id) {
                None => Ok(()),
                Some(ResponseFormat::Text) => Ok(()),
                Some(_) => Err(super::Error::UnsupportedResponseFormat),
            }
        }
        Some(_) => Err(super::Error::UnsupportedResponseFormat),
    }
}

impl UpstreamClient<objectiveai::agent::claude_agent_sdk::Agent, objectiveai::agent::claude_agent_sdk::Continuation> for Client {
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
        agent: &objectiveai::agent::claude_agent_sdk::Agent,
        request_continuation: Option<&objectiveai::agent::claude_agent_sdk::Continuation>,
        params: &objectiveai::agent::completions::request::AgentCompletionCreateParams,
        messages: &[objectiveai::agent::completions::message::Message],
        mcp_connections: &[Arc<crate::mcp::Connection>],
        invention_tools: Option<
            &[objectiveai::functions::inventions::InventionTool],
        >,
        tool_names: &[String],
        tool_map: &std::collections::HashMap<String, super::super::tool::ResolvedTool>,
        continuation: Option<&[ContinuationItem<Self::State>]>,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        _tools_enabled: bool,
        _invention_type: Option<objectiveai::functions::inventions::prompts::StepPromptType>,
        _invention_step: Option<usize>,
        _invention_tasks_min: Option<u64>,
        _invention_input_schema: Option<String>,
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static {
        let enabled = self.enabled;
        let tools_enabled = _tools_enabled;
        let has_tools = !tool_names.is_empty();
        let is_byok = byok.is_some();
        let id = id.to_string();
        let agent = agent.clone();
        let params = params.clone();
        let messages = messages.to_vec();
        let mcp_connections = mcp_connections.to_vec();
        let invention_tools = invention_tools.map(|t| t.to_vec());
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

            if !tools_enabled && has_tools {
                return Err(super::Error::ToolsNotAllowed);
            }

            validate_response_format(&agent.id, &params.response_format)?;

            // Build prompt from messages + continuation (handles continuation validation).
            let prompt = Prompt::new(&messages, continuation.as_deref(), request_continuation.as_ref())?;

            // Spawn invention server if invention tools are provided.
            let invention_server = if let Some(ref tools) = invention_tools {
                if !tools.is_empty() {
                    Some(InventionServer::new(tools.clone()).await)
                } else {
                    None
                }
            } else {
                None
            };

            // Serialize message and MCP servers for CLI args.
            let message_json = serde_json::to_string(&prompt.message)
                .map_err(|e| super::Error::Json(e.to_string()))?;
            let mcp_servers_json =
                build_mcp_servers_json(&mcp_connections, invention_server.as_ref());

            // Compute assistant_index from continuation.
            // State items carry a message_count (may be >1 since the SDK
            // handles its own multi-turn loop). Other items count as 1.
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

            let binary = client.binary_path()
                .ok_or_else(|| super::Error::Spawn(
                    "failed to extract claude-agent-sdk-runner binary".to_string(),
                ))?
                .to_owned();
            let agent_id = agent.id.clone();

            // Spawn the Python-based runner binary with CLI args.
            let mut cmd = Command::new(&binary);
            cmd.arg("--model").arg(&agent.base.model)
                .arg("--message").arg(&message_json);

            if let Some(s) = &prompt.system_prompt {
                cmd.arg("--system-prompt").arg(s);
            }
            if let Some(e) = agent.base.effort {
                cmd.arg("--effort").arg(e.as_str());
            }
            if agent.base.thinking == Some(false) {
                cmd.arg("--thinking-disabled");
            }
            if mcp_servers_json != "{}" {
                cmd.arg("--mcp-servers").arg(&mcp_servers_json);
            }
            let session_id = &prompt.message.session_id;
            if !session_id.is_empty() {
                cmd.arg("--resume").arg(session_id);
            }
            cmd.arg("--user-agent").arg(&client.user_agent);
            cmd.arg("--rate-limit-max-retries").arg(client.rate_limit_max_retries.to_string());

            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            cmd.env_remove("CLAUDECODE");

            let mut child = cmd.spawn().map_err(|e| {
                super::Error::Spawn(e.to_string())
            })?;

            // Collect stderr in background.
            let stderr = child.stderr.take().expect("stderr was piped");
            let stderr_handle = tokio::spawn(async move {
                let mut buf = String::new();
                let mut reader = BufReader::new(stderr);
                let _ = tokio::io::AsyncReadExt::read_to_string(&mut reader, &mut buf).await;
                buf
            });

            // Read stdout lines.
            let stdout = child.stdout.take().expect("stdout was piped");
            let reader = BufReader::new(stdout);
            let mut lines_stream = LinesStream::new(reader.lines());

            let id_for_peek = id.clone();
            let internal_stream = async_stream::stream! {
                // Keep invention server alive for the duration of the stream.
                let _invention_server_guard = invention_server;

                let mut latest_session_id = String::new();
                let mut had_error = false;
                let mut msg_index = assistant_index;

                loop {
                    match lines_stream.next().await {
                        None => {
                            // Process ended — collect stderr.
                            let stderr_ctx = stderr_handle.await
                                .ok()
                                .unwrap_or_default();

                            if !stderr_ctx.is_empty() {
                                yield Err(
                                    super::Error::Stderr(stderr_ctx.trim().to_owned()),
                                );
                                had_error = true;
                            }
                            break;
                        }
                        Some(Err(e)) => {
                            let _ = child.kill().await;
                            yield Err(
                                super::Error::Io(e.to_string()),
                            );
                            had_error = true;
                            break;
                        }
                        Some(Ok(line)) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }

                            let sdk_msg: SDKMessage = match serde_json::from_str(trimmed) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    // Log deserialization errors but continue — unknown
                                    // message types are expected as the SDK evolves.
                                    continue;
                                }
                            };

                            // Track latest session_id.
                            if let Some(sid) = sdk_msg.session_id() {
                                if !sid.is_empty() {
                                    latest_session_id = sid.to_string();
                                }
                            }

                            match sdk_msg.into_downstream(
                                id.clone(),
                                created,
                                agent_id.clone(),
                                msg_index,
                                is_byok,
                                cost_multiplier,
                            ) {
                                Some(Ok(chunk)) => {
                                    // Advance the index when a message slot is
                                    // complete: an assistant turn with a finish
                                    // reason, or a tool response.
                                    use objectiveai::agent::completions::response::streaming::MessageChunk;
                                    let advances_index = chunk.messages.iter().any(|m| match m {
                                        MessageChunk::Assistant(a) => a.finish_reason.is_some(),
                                        MessageChunk::Tool(_) => true,
                                    });
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
                    }
                }

                if !had_error {
                    // Yield final state with session_id and message count.
                    yield Ok(StreamItem::State(super::State {
                        session_id: latest_session_id,
                        message_count: msg_index - assistant_index,
                    }));
                }
            };

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
                    let id_for_stream = id_for_peek.clone();
                    let rest = stream.map(move |item| match item {
                        Ok(si) => si,
                        Err(e) => {
                            use objectiveai::error::StatusError;
                            StreamItem::Chunk(
                                objectiveai::agent::completions::response::streaming::AgentCompletionChunk {
                                    id: id_for_stream.clone(),
                                    error: Some(objectiveai::error::ResponseError {
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
                    return Err(super::Error::NoOutput);
                }
            }
        }
    }

    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&objectiveai::agent::claude_agent_sdk::Continuation>,
        _messages: &[objectiveai::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
    ) -> objectiveai::agent::claude_agent_sdk::Continuation {
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

        objectiveai::agent::claude_agent_sdk::Continuation {
            upstream: objectiveai::agent::claude_agent_sdk::Upstream::default(),
            session_id,
            mcp_sessions,
        }
    }
}
