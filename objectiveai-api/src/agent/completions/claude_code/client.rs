use std::pin::Pin;
use std::sync::Arc;
use futures::{Stream, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::LinesStream;

use super::super::{ContinuationItem, StreamItem, UpstreamClient};
use super::sdk_message::{RateLimitEventType, RateLimitStatus, SDKMessage};
use super::invention_server::InventionServer;
use super::mcp_server_config::McpHttpServerConfig;
use crate::util::StreamOnce;

// Reuse the Claude Agent SDK's Prompt — same stream-json wire format.
// (SDK is just a wrapper around claude; both use identical message format.)
use super::super::claude_agent_sdk::prompt::Prompt;

/// The full list of Claude Code's built-in tools that we disable by default.
/// Source: https://code.claude.com/docs/en/tools-reference
const DISALLOWED_TOOLS: &[&str] = &[
    "Agent",
    "AskUserQuestion",
    "Bash",
    "CronCreate",
    "CronDelete",
    "CronList",
    "Edit",
    "EnterPlanMode",
    "EnterWorktree",
    "ExitPlanMode",
    "ExitWorktree",
    "Glob",
    "Grep",
    "ListMcpResourcesTool",
    "LSP",
    "Monitor",
    "NotebookEdit",
    "PowerShell",
    "Read",
    "ReadMcpResourceTool",
    "SendMessage",
    "Skill",
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskOutput",
    "TaskStop",
    "TaskUpdate",
    "TeamCreate",
    "TeamDelete",
    "TodoWrite",
    "ToolSearch",
    "WebFetch",
    "WebSearch",
    "Write",
];

/// Claude Code client for agent completions.
///
/// Spawns the `claude` binary directly from the host's PATH. Unlike the
/// Claude Agent SDK client, this does not embed or extract a runner — the
/// user is expected to have Claude Code installed on their system.
#[derive(Debug, Clone)]
pub struct Client {
    pub user_agent: String,
    pub enabled: bool,
    pub rate_limit_max_retries: u64,
    pub rate_limit_max_wait_secs: u64,
}

impl Client {
    pub fn new(user_agent: String, enabled: bool, rate_limit_max_retries: u64, rate_limit_max_wait_secs: u64) -> Self {
        Self { user_agent, enabled, rate_limit_max_retries, rate_limit_max_wait_secs }
    }
}

/// Builds the MCP servers JSON config for claude's `--mcp-config` flag.
/// claude expects `{"mcpServers": {"name": {...}, ...}}`.
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
    serde_json::to_string(&serde_json::json!({ "mcpServers": servers })).unwrap()
}

fn validate_response_format(
    agent_id: &str,
    response_format: &Option<objectiveai::agent::completions::request::ResponseFormatParam>,
) -> Result<(), super::Error> {
    use objectiveai::agent::completions::request::{ResponseFormat, ResponseFormatParam};
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

impl UpstreamClient<
    objectiveai::agent::claude_code::Agent,
    objectiveai::agent::claude_code::Continuation,
> for Client {
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
        agent: &objectiveai::agent::claude_code::Agent,
        request_continuation: Option<&objectiveai::agent::claude_code::Continuation>,
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
        let request_continuation_cc = request_continuation.cloned();
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

            // Build prompt via SDK's Prompt::new — this handles rich content,
            // continuation reconstruction, name prefixing, multiple system
            // messages, and rejects audio/video with InvalidMessages.
            //
            // We need to adapt: claude_code's State is identical to
            // claude_agent_sdk's State (re-exported), so ContinuationItem<State>
            // is compatible. But Prompt::new takes claude_agent_sdk::Continuation
            // — we need to convert from claude_code::Continuation.
            let request_continuation_cas = request_continuation_cc.as_ref().map(|cc| {
                objectiveai::agent::claude_agent_sdk::Continuation {
                    upstream: objectiveai::agent::claude_agent_sdk::Upstream::default(),
                    session_id: cc.session_id.clone(),
                    mcp_sessions: cc.mcp_sessions.clone(),
                }
            });
            let prompt = Prompt::new(
                &messages,
                continuation.as_deref(),
                request_continuation_cas.as_ref(),
            ).map_err(|e| translate_sdk_error(&e.to_string()))?;

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

            let mcp_servers_json =
                build_mcp_servers_json(&mcp_connections, invention_server.as_ref());

            // Compute assistant_index from continuation.
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

            let agent_id = agent.id.clone();
            let initial_session_id = prompt.message.session_id.clone();
            let model = agent.base.model.clone();
            let system_prompt = prompt.system_prompt.clone();
            let user_agent = client.user_agent.clone();
            let rate_limit_max_retries = client.rate_limit_max_retries;
            let rate_limit_max_wait_secs = client.rate_limit_max_wait_secs;
            let has_mcp_config = !mcp_connections.is_empty() || invention_server.is_some();

            // Serialize the SDKUserMessage once — stdin content is identical
            // across spawn attempts (resume is controlled via --resume).
            let message_json = serde_json::to_string(&prompt.message)
                .map_err(|e| super::Error::Json(e.to_string()))?;

            let id_for_peek = id.clone();
            let our_mcp_server_names: std::collections::HashSet<String> = mcp_connections
                .iter()
                .map(|c| c.initialize_result.server_info.name.clone())
                .chain(invention_server.as_ref().map(|_| "objectiveai-invention".to_string()))
                .collect();

            // Builds a fresh `claude` Command. Called once per spawn attempt so
            // that we can re-spawn cleanly after a rate-limit wait.
            let spawn_cmd = move |session_override: &str| -> Command {
                let mut cmd = Command::new("claude");
                cmd.arg("--input-format").arg("stream-json")
                    .arg("--output-format").arg("stream-json")
                    .arg("--verbose")
                    .arg("--include-partial-messages")
                    .arg("-p")
                    .arg("--permission-mode").arg("bypassPermissions")
                    .arg("--model").arg(&model)
                    .arg("--disallowed-tools").arg(DISALLOWED_TOOLS.join(","));
                if let Some(sp) = &system_prompt {
                    cmd.arg("--system-prompt").arg(sp);
                }
                if has_mcp_config {
                    cmd.arg("--mcp-config").arg(&mcp_servers_json);
                }
                if !session_override.is_empty() {
                    cmd.arg("--resume").arg(session_override);
                }
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
                cmd.env_remove("CLAUDECODE");
                if !user_agent.is_empty() {
                    cmd.env("CLAUDE_CODE_CLIENT_APP", &user_agent);
                }
                cmd
            };

            let internal_stream = async_stream::stream! {
                let _invention_server_guard = invention_server;

                let mut latest_session_id = String::new();
                let mut msg_index = assistant_index;
                let mut had_error = false;
                let mut retries: u64 = 0;
                // Most-recent assistant index, so the SDK's trailing
                // ResultMessage (a usage/cost summary, not a real second
                // turn) can re-use it. Per protocol, assistant messages
                // never sit back-to-back at distinct indices — they
                // alternate with tool messages — so the trailer must
                // merge into the assistant that just finished, not stand
                // as its own message at the next index.
                let mut last_assistant_index: Option<u64> = None;

                // Rate-limit retry loop. Each iteration spawns a fresh
                // `claude` subprocess. If a rate_limit_event with status
                // "rejected" arrives, we kill the child, sleep until
                // resets_at, and spawn again (resuming the session).
                'retry: loop {
                    let session_for_spawn = if !latest_session_id.is_empty() {
                        latest_session_id.clone()
                    } else {
                        initial_session_id.clone()
                    };

                    let mut cmd = spawn_cmd(&session_for_spawn);
                    let mut child = match cmd.spawn() {
                        Ok(c) => c,
                        Err(e) => {
                            yield Err(super::Error::Spawn(e.to_string()));
                            had_error = true;
                            break 'retry;
                        }
                    };

                    if let Some(mut stdin) = child.stdin.take() {
                        let line = format!("{}\n", message_json);
                        if let Err(e) = stdin.write_all(line.as_bytes()).await {
                            let _ = child.kill().await;
                            yield Err(super::Error::Io(e.to_string()));
                            had_error = true;
                            break 'retry;
                        }
                        stdin.shutdown().await.ok();
                        drop(stdin);
                    }

                    let stderr = child.stderr.take().expect("stderr was piped");
                    let stderr_handle = tokio::spawn(async move {
                        let mut buf = String::new();
                        let mut reader = BufReader::new(stderr);
                        let _ = tokio::io::AsyncReadExt::read_to_string(
                            &mut reader, &mut buf
                        ).await;
                        buf
                    });

                    let stdout = child.stdout.take().expect("stdout was piped");
                    let reader = BufReader::new(stdout);
                    let mut lines_stream = LinesStream::new(reader.lines());

                    let mut saw_init = false;
                    // When set, break out of the line loop to retry after
                    // sleeping until this Unix-seconds instant.
                    let mut rate_limited_resets_at: Option<u64> = None;

                    loop {
                        match lines_stream.next().await {
                            None => {
                                let stderr_ctx = stderr_handle.await.ok().unwrap_or_default();
                                if !stderr_ctx.is_empty() {
                                    yield Err(super::Error::Stderr(stderr_ctx.trim().to_owned()));
                                    had_error = true;
                                }
                                break;
                            }
                            Some(Err(e)) => {
                                let _ = child.kill().await;
                                yield Err(super::Error::Io(e.to_string()));
                                had_error = true;
                                break;
                            }
                            Some(Ok(line)) => {
                                let trimmed = line.trim();
                                if trimmed.is_empty() {
                                    continue;
                                }

                                if !saw_init {
                                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                                        if value.get("type") == Some(&serde_json::Value::String("system".to_string()))
                                            && value.get("subtype") == Some(&serde_json::Value::String("init".to_string()))
                                        {
                                            saw_init = true;
                                            if let Err(e) = check_mcp_servers(&value, &our_mcp_server_names) {
                                                yield Err(e);
                                                had_error = true;
                                                break;
                                            }
                                        }
                                    }
                                }

                                let sdk_msg: SDKMessage = match serde_json::from_str(trimmed) {
                                    Ok(msg) => msg,
                                    Err(_) => continue,
                                };

                                if let Some(sid) = sdk_msg.session_id() {
                                    if !sid.is_empty() {
                                        latest_session_id = sid.to_string();
                                    }
                                }

                                // Intercept rate-limit events before
                                // `into_downstream` converts them to an error.
                                // Only retry on status = "rejected" with a
                                // known resets_at; otherwise fall through.
                                if let SDKMessage::RateLimitEvent(ref evt) = sdk_msg {
                                    let rejected = evt
                                        .rate_limit_info
                                        .and_then(|i| i.status)
                                        .map(|s| matches!(s, RateLimitStatus::Rejected))
                                        .unwrap_or(false);
                                    let resets = evt
                                        .rate_limit_info
                                        .and_then(|i| i.resets_at);
                                    // Also treat `r#type == RateLimit` as a
                                    // terminal rate-limit signal if we don't
                                    // have info — upstream tests emit it.
                                    let terminal_type = matches!(
                                        evt.r#type,
                                        RateLimitEventType::RateLimit
                                    );
                                    if rejected || terminal_type {
                                        rate_limited_resets_at = resets;
                                        break;
                                    }
                                }

                                // ResultMessage is the SDK's end-of-stream
                                // usage/cost trailer, not a fresh assistant
                                // turn — it must merge into the previous
                                // assistant's message slot, not occupy the
                                // next one. Use that assistant's index.
                                let effective_index = match &sdk_msg {
                                    SDKMessage::ResultMessage(_) => {
                                        last_assistant_index.unwrap_or(msg_index)
                                    }
                                    _ => msg_index,
                                };

                                match sdk_msg.into_downstream(
                                    id.clone(),
                                    created,
                                    agent_id.clone(),
                                    effective_index,
                                    is_byok,
                                    cost_multiplier,
                                    objectiveai::agent::Upstream::ClaudeCode,
                                ) {
                                    Some(Ok(chunk)) => {
                                        // Track the index of any assistant in
                                        // this chunk and decide whether to
                                        // advance to the next message slot.
                                        // Tool messages always advance; assistant
                                        // messages advance only on finish_reason.
                                        use objectiveai::agent::completions::response::streaming::MessageChunk;
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
                                    Some(Err(sdk_err)) => {
                                        yield Err(translate_sdk_error(&sdk_err.to_string()));
                                        had_error = true;
                                        break;
                                    }
                                    None => {}
                                }
                            }
                        }
                    }

                    // If we exited the inner loop due to rate limit, sleep
                    // until resets_at and retry — up to rate_limit_max_retries
                    // and capped by rate_limit_max_wait_secs.
                    if let Some(resets_at) = rate_limited_resets_at {
                        let _ = child.kill().await;
                        if retries >= rate_limit_max_retries {
                            yield Err(super::Error::RateLimit);
                            had_error = true;
                            break 'retry;
                        }
                        let now_secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let wait_secs = resets_at
                            .saturating_sub(now_secs)
                            .saturating_add(1);
                        if wait_secs > rate_limit_max_wait_secs {
                            // The rate-limit window is longer than we're
                            // willing to wait — give up instead of sleeping.
                            yield Err(super::Error::RateLimit);
                            had_error = true;
                            break 'retry;
                        }
                        retries += 1;
                        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                        continue 'retry;
                    }

                    // Normal completion (or non-rate-limit error).
                    break 'retry;
                }

                if !had_error {
                    yield Ok(StreamItem::State(super::State {
                        session_id: latest_session_id,
                        message_count: msg_index - assistant_index,
                    }));
                }
            };

            let mut stream = Box::pin(internal_stream);
            match stream.next().await {
                Some(Err(e)) => Err(e),
                Some(Ok(first)) => {
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
                None => Err(super::Error::NoOutput),
            }
        }
    }

    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&objectiveai::agent::claude_code::Continuation>,
        _messages: &[objectiveai::agent::completions::message::Message],
        continuation: Option<&[ContinuationItem<Self::State>]>,
    ) -> objectiveai::agent::claude_code::Continuation {
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

        objectiveai::agent::claude_code::Continuation {
            upstream: objectiveai::agent::claude_code::Upstream::default(),
            session_id,
            mcp_sessions,
        }
    }
}

/// Translate Claude Agent SDK error messages (from reused Prompt/SDKMessage code)
/// to this module's Error type. The SDK's Error type has equivalent variants.
fn translate_sdk_error(msg: &str) -> super::Error {
    let s = msg.to_string();
    if s.contains("invalid continuation") {
        super::Error::InvalidContinuation(s)
    } else if s.contains("invalid messages") || s.contains("unsupported content type") {
        super::Error::InvalidMessages(s)
    } else if s.contains("rate limited") {
        super::Error::RateLimit
    } else {
        super::Error::InvalidMessages(s)
    }
}

/// Parse the system/init event's mcp_servers array and error if any of our
/// configured servers failed to connect.
fn check_mcp_servers(
    init_event: &serde_json::Value,
    our_servers: &std::collections::HashSet<String>,
) -> Result<(), super::Error> {
    if our_servers.is_empty() {
        return Ok(());
    }

    let servers = match init_event.get("mcp_servers").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(()),
    };

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for server in servers {
        let name = match server.get("name").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        if !our_servers.contains(name) {
            continue;
        }
        seen.insert(name.to_string());
        let status = server
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match status {
            "connected" | "pending" => {}
            "failed" | "needs-auth" => {
                let err = server
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return Err(super::Error::McpServer(format!(
                    "MCP server {name}: {status}{}",
                    if err.is_empty() { "".to_string() } else { format!(" - {err}") }
                )));
            }
            _ => {}
        }
    }

    let missing: Vec<String> = our_servers
        .iter()
        .filter(|n| !seen.contains(*n))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(super::Error::McpServer(format!(
            "MCP servers not found in init status: {}",
            missing.join(", ")
        )));
    }

    Ok(())
}
