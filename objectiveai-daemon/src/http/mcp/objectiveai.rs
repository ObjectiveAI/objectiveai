use std::sync::Arc;

use futures::StreamExt;
use objectiveai_sdk::cli::Error as CliError;
use objectiveai_sdk::cli::ErrorType as CliErrorType;
use objectiveai_sdk::cli::Level as CliLevel;
use objectiveai_sdk::identity::Identity;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::CommandRequest;
use objectiveai_sdk::cli::command::CommandResponse;
use objectiveai_sdk::cli::command::McpResponseItem;
use objectiveai_sdk::cli::command::Request;
use objectiveai_sdk::cli::command::ResponseItem;
use objectiveai_sdk::cli::command::RequestBase;
use objectiveai_sdk::cli::command::Transform;
use objectiveai_sdk::cli::command::parse_request;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Extension,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_router,
};

use super::agent_args_registry::IdentityRegistry;
use super::format::format_items;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ObjectiveAiRequest {
    #[schemars(
        description = "The command arguments to pass to the ObjectiveAI CLI (e.g. [\"agents\", \"list\"] or [\"functions\", \"executions\", \"create\", \"--help\"])"
    )]
    pub command: Vec<String>,
    #[schemars(
        description = "Timeout for the whole command, humantime (e.g. \"30s\", \"5m\", \"1h30m\")."
    )]
    pub timeout: String,
    #[schemars(description = "Output token budget for the response.")]
    pub max_tokens: u64,
    #[schemars(
        description = "Optional jq filter applied to each output line. Return null to discard the output line. Ignored when `python` is also set."
    )]
    pub jq: Option<String>,
    #[schemars(
        description = "Optional Python transform applied to each output line. The item arrives as the global `input`; print the transformed result as valid JSON, or return null to discard the output line. Overrides `jq` when both are set."
    )]
    pub python: Option<String>,
}

/// Validate the shared `timeout` / `max_tokens` tool arguments. Parses
/// the humantime `timeout` to whole seconds (erroring on a bad string
/// or a sub-second-rounds-to-zero value) and rejects a zero
/// `max_tokens`. Returns `(timeout_seconds, max_tokens)`.
fn parse_caps(timeout: &str, max_tokens: u64) -> Result<(u64, u64), String> {
    let secs = humantime::parse_duration(timeout)
        .map_err(|e| format!("invalid timeout {timeout:?}: {e}"))?
        .as_secs();
    if secs == 0 {
        return Err("timeout must be >= 1s".to_string());
    }
    if max_tokens == 0 {
        return Err("max_tokens must be >= 1".to_string());
    }
    Ok((secs, max_tokens))
}

/// The active output transform from the tool args — python overrides
/// jq, matching `RequestBase::transform`.
fn build_transform(jq: Option<String>, python: Option<String>) -> Option<Transform> {
    if let Some(code) = python {
        Some(Transform::Python(code))
    } else {
        jq.map(Transform::Jq)
    }
}

#[derive(Debug)]
pub struct ObjectiveAiMcpCli<E> {
    pub tool_router: ToolRouter<Self>,
    pub executor: Arc<E>,
    /// Per-rmcp-session bag of [`Identity`] captured from the
    /// six `X-OBJECTIVEAI-*` request headers at `initialize` time.
    /// Tool dispatchers look up the inbound `Mcp-Session-Id` against
    /// this registry to recover the caller's identity — request
    /// headers on non-initialize calls are intentionally ignored.
    pub registry: Arc<IdentityRegistry>,
}

impl<E> Clone for ObjectiveAiMcpCli<E> {
    fn clone(&self) -> Self {
        Self {
            tool_router: self.tool_router.clone(),
            executor: self.executor.clone(),
            registry: self.registry.clone(),
        }
    }
}

#[tool_router]
impl<E> ObjectiveAiMcpCli<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    /// Build the handler: the single static `ObjectiveAI` catch-all
    /// tool over the given executor. (The per-plugin dynamic tool
    /// routes were removed with the "installed plugins" surface.)
    pub fn new(executor: Arc<E>, registry: Arc<IdentityRegistry>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            executor,
            registry,
        }
    }

    #[tool(name = "ObjectiveAI", description = "Run an ObjectiveAI command.")]
    async fn objectiveai(
        &self,
        Parameters(req): Parameters<ObjectiveAiRequest>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // Validate the caps, parse the command, then inject the caps
        // directly onto the parsed request's base — no argv round-trip.
        // The transform is passed to dispatch (not into the base) — its
        // leaf `execute_transform` sets it on the base itself.
        let (timeout_seconds, max_tokens) = match parse_caps(&req.timeout, req.max_tokens) {
            Ok(v) => v,
            Err(msg) => {
                let item = synthetic_error(msg).into_mcp();
                return Ok(CallToolResult::success(format_items(vec![item])));
            }
        };
        let mut request = match parse_request(&req.command) {
            Ok(r) => r,
            Err(e) => {
                let item = synthetic_error(e.to_string()).into_mcp();
                return Ok(CallToolResult::success(format_items(vec![item])));
            }
        };
        if let Some(base) = request.request_base_mut() {
            base.timeout_seconds = Some(timeout_seconds);
            base.max_tokens = Some(max_tokens);
        }
        let transform = build_transform(req.jq, req.python);
        let session_id = session_id_from_headers(&parts.headers);
        let state = match session_id {
            Some(sid) => self.registry.get(&sid.into()).await,
            None => None,
        };
        let blocks = dispatch_root(
            &*self.executor,
            request,
            transform,
            state.as_deref().map(|s| &s.args),
        )
        .await;
        Ok(CallToolResult::success(blocks))
    }
}

/// Pull `Mcp-Session-Id` out of an inbound HTTP request's headers
/// (case-insensitive, trimmed, empty → `None`).
fn session_id_from_headers(headers: &http::HeaderMap) -> Option<String> {
    headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Same as [`session_id_from_headers`] but reads through the
/// `http::request::Parts` injected into rmcp's request-context
/// extensions. Used by the dynamic plugin/tool routes whose
/// closures take a [`rmcp::handler::server::tool::ToolCallContext`]
/// rather than an [`rmcp::handler::server::tool::Extension`]
/// extractor.
fn session_id_from_extensions(extensions: &rmcp::model::Extensions) -> Option<String> {
    extensions
        .get::<http::request::Parts>()
        .and_then(|p| session_id_from_headers(&p.headers))
}

async fn dispatch_root<E>(
    executor: &E,
    request: Request,
    transform: Option<Transform>,
    identity: Option<&Identity>,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    match transform {
        None => {
            let stream =
                match objectiveai_sdk::cli::command::execute(executor, request, identity)
                    .await
                {
                    Ok(s) => s,
                    Err(e) => return format_items(vec![convert::<ResponseItem, _>(Err(e))]),
                };
            let items: Vec<McpResponseItem> =
                stream.map(convert::<ResponseItem, _>).collect().await;
            format_items(items)
        }
        Some(t) => {
            let stream = match objectiveai_sdk::cli::command::execute_transform(
                executor,
                request,
                t,
                identity,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => return format_items(vec![convert_value(Err(e))]),
            };
            let items: Vec<McpResponseItem> = stream.map(convert_value).collect().await;
            format_items(items)
        }
    }
}

/// Collapse a `Result<T, ExecErr>` (the executor's per-item shape)
/// into an `McpResponseItem`. The executor's error gets formatted
/// via `Display` into a synthetic `cli::Error` so it renders through
/// the same `Result<T, cli::Error>: CommandResponse` path.
fn convert<T: CommandResponse, ExecErr: std::fmt::Display>(
    r: Result<T, ExecErr>,
) -> McpResponseItem {
    let result: Result<T, CliError> = r.map_err(|e| synthetic_error(format!("{e}")));
    result.into_mcp()
}

/// Like [`convert`], but for the raw `serde_json::Value` items an
/// `execute_transform` stream yields (the transform replaced the typed
/// shape). An `Ok` value rides through as JSONL; an executor error
/// becomes a synthetic `cli::Error`.
fn convert_value<ExecErr: std::fmt::Display>(
    r: Result<serde_json::Value, ExecErr>,
) -> McpResponseItem {
    match r {
        Ok(value) => McpResponseItem::JSONL(value),
        Err(e) => synthetic_error(format!("{e}")).into_mcp(),
    }
}

/// Build a `cli::Error` envelope from a free-form message. Used at the
/// pre-dispatch failure sites (clap parse errors, `TryFrom<Command>`
/// errors) and as the wrapper for non-`Cli` `binary::Error` variants.
fn synthetic_error(message: impl Into<String>) -> CliError {
    CliError {
        r#type: CliErrorType::Error,
        level: Some(CliLevel::Error),
        fatal: Some(true),
        message: serde_json::Value::String(message.into()),
    }
}

// Hand-written `ServerHandler` impl, replacing `#[tool_handler]`.
// `call_tool` and `get_tool` are byte-identical copies of what the
// macro emits (see `rmcp-macros::tool_handler`). `list_tools` is the
// custom bit: it gates the `ObjectiveAI` tool on the per-session
// `mcp_root` flag stamped by `header_session_manager`.
//
// Classification in `list_tools`: `ObjectiveAI` (the only registered
// tool) is gated on the per-session `mcp_root`. No filter recorded for
// the session (no header parser ran, e.g. GET-only flow) ⇒ behave as
// `root=true` — the tool is advertised.
impl<E> ServerHandler for ObjectiveAiMcpCli<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`,
        // so build via `Default` + explicit field assignment.
        let mut server_info = Implementation::default();
        server_info.name = "oai".into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = None;
        info
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let session_id = session_id_from_extensions(&context.extensions);
        let state = match session_id {
            Some(sid) => self.registry.get(&sid.into()).await,
            None => None,
        };
        let root = state.as_deref().map(|s| s.mcp_root).unwrap_or(true);

        let tools = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|t| {
                if t.name.as_ref() == "ObjectiveAI" {
                    return root;
                }
                true
            })
            .collect();

        Ok(rmcp::model::ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    fn get_tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        self.tool_router.get(name).cloned()
    }
}
