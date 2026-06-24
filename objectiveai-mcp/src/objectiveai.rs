use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use futures::FutureExt;
use futures::StreamExt;
use objectiveai_sdk::agent::ClientObjectiveaiMcpEntry;
use objectiveai_sdk::cli::Error as CliError;
use objectiveai_sdk::cli::ErrorType as CliErrorType;
use objectiveai_sdk::cli::Level as CliLevel;
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::CommandRequest;
use objectiveai_sdk::cli::command::CommandResponse;
use objectiveai_sdk::cli::command::McpResponseItem;
use objectiveai_sdk::cli::command::Request;
use objectiveai_sdk::cli::command::ResponseItem;
use objectiveai_sdk::cli::command::RequestBase;
use objectiveai_sdk::cli::command::Transform;
use objectiveai_sdk::cli::command::parse_request;
use objectiveai_sdk::cli::command::plugins;
use objectiveai_sdk::cli::command::tools;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::{Extension, parse_json_object, schema_for_type},
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    },
    schemars, tool, tool_router,
};

use crate::agent_args_registry::AgentArgumentsRegistry;
use crate::format::format_items;

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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PluginRequest {
    #[schemars(description = "Args forwarded to the plugin binary's argv.")]
    pub args: Vec<String>,
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ToolRequest {
    #[schemars(description = "Args appended verbatim to the tool's exec command.")]
    pub args: Vec<String>,
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
    /// Per-rmcp-session bag of [`AgentArguments`] captured from the
    /// six `X-OBJECTIVEAI-*` request headers at `initialize` time.
    /// Tool dispatchers look up the inbound `Mcp-Session-Id` against
    /// this registry to recover the caller's identity — request
    /// headers on non-initialize calls are intentionally ignored.
    pub registry: Arc<AgentArgumentsRegistry>,
    /// Tool-name → manifest triple for every CLI tool registered as
    /// a dynamic route. Used by the hand-written `list_tools`
    /// handler to classify each routed tool by origin and apply the
    /// per-session `X-OBJECTIVEAI-MCP-TOOLS` filter.
    pub tools_by_tool_name: HashMap<String, ClientObjectiveaiMcpEntry>,
    /// Same as `tools_by_tool_name`, but for CLI plugins. A
    /// tool-name collision between a plugin and a CLI tool ends up
    /// classified as a *tool*: the existing `with_plugins_and_tools`
    /// loop registers plugins first then tools, so
    /// `tool_router.add_route` last-writer-wins makes the live
    /// route a tool. The hand-written `list_tools` mirrors this by
    /// checking `tools_by_tool_name` first.
    pub plugins_by_tool_name: HashMap<String, ClientObjectiveaiMcpEntry>,
}

impl<E> Clone for ObjectiveAiMcpCli<E> {
    fn clone(&self) -> Self {
        Self {
            tool_router: self.tool_router.clone(),
            executor: self.executor.clone(),
            registry: self.registry.clone(),
            tools_by_tool_name: self.tools_by_tool_name.clone(),
            plugins_by_tool_name: self.plugins_by_tool_name.clone(),
        }
    }
}

#[tool_router]
impl<E> ObjectiveAiMcpCli<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    /// Build a handler with one dynamic tool per discovered CLI plugin
    /// and CLI tool, plus the static `ObjectiveAI` catch-all. Plugins
    /// and tools are listed once at server startup (see `run::setup`);
    /// this constructor is not re-invoked when either is added later,
    /// so hot reload is intentionally out of scope.
    ///
    /// Name collisions: if a CLI plugin and a CLI tool happen to share
    /// a name (or with `ObjectiveAI`), the plugin registers first and
    /// the tool's `add_route` overwrites it (last-writer-wins) — but
    /// `ObjectiveAI` itself is always skipped on both sides so the
    /// built-in catch-all is never shadowed.
    pub fn with_plugins_and_tools(
        executor: Arc<E>,
        plugins_list: Vec<plugins::list::ResponseItem>,
        tools_list: Vec<tools::list::ResponseItem>,
        registry: Arc<AgentArgumentsRegistry>,
    ) -> Self {
        let mut tool_router = Self::tool_router();
        let mut plugins_by_tool_name: HashMap<String, ClientObjectiveaiMcpEntry> =
            HashMap::new();
        let mut tools_by_tool_name: HashMap<String, ClientObjectiveaiMcpEntry> =
            HashMap::new();
        for plugin in plugins_list {
            if plugin.name == "ObjectiveAI" {
                continue;
            }
            plugins_by_tool_name.insert(
                plugin.tool_name(),
                ClientObjectiveaiMcpEntry {
                    owner: plugin.owner.clone(),
                    name: plugin.name.clone(),
                    version: plugin.version.clone(),
                },
            );
            let plugin_owner = plugin.owner.clone();
            let plugin_name = plugin.name.clone();
            let plugin_version = plugin.version.clone();
            let executor_for_route = executor.clone();
            let tool = Tool::new(
                Cow::Owned(plugin.tool_name()),
                Cow::Owned(plugin.description.clone()),
                schema_for_type::<PluginRequest>(),
            );
            let registry_for_route = registry.clone();
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let executor = executor_for_route.clone();
                let plugin_owner = plugin_owner.clone();
                let plugin_name = plugin_name.clone();
                let plugin_version = plugin_version.clone();
                let registry = registry_for_route.clone();
                let session_id = session_id_from_extensions(&ctx.request_context.extensions);
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: PluginRequest = parse_json_object(arguments)?;
                    let (timeout_seconds, max_tokens) =
                        match parse_caps(&req.timeout, req.max_tokens) {
                            Ok(v) => v,
                            Err(msg) => {
                                let item = synthetic_error(msg).into_mcp();
                                return Ok(CallToolResult::success(format_items(vec![item])));
                            }
                        };
                    let transform = build_transform(req.jq, req.python);
                    let request = plugins::run::Request {
                        path_type: plugins::run::Path::PluginsRun,
                        owner: plugin_owner,
                        name: plugin_name,
                        version: plugin_version,
                        args: req.args,
                        base: RequestBase {
                            jq: None,
                            python: None,
                            timeout_seconds: Some(timeout_seconds),
                            max_tokens: Some(max_tokens),
                        },
                    };
                    let state = match session_id {
                        Some(sid) => registry.get(&sid.into()).await,
                        None => None,
                    };
                    let blocks = dispatch_plugins_run(
                        &*executor,
                        request,
                        transform,
                        state.as_deref().map(|s| &s.args),
                    )
                    .await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        for cli_tool in tools_list {
            if cli_tool.name == "ObjectiveAI" {
                continue;
            }
            tools_by_tool_name.insert(
                cli_tool.tool_name(),
                ClientObjectiveaiMcpEntry {
                    owner: cli_tool.owner.clone(),
                    name: cli_tool.name.clone(),
                    version: cli_tool.version.clone(),
                },
            );
            let tool_owner = cli_tool.owner.clone();
            let tool_name = cli_tool.name.clone();
            let tool_version = cli_tool.version.clone();
            let executor_for_route = executor.clone();
            let tool = Tool::new(
                Cow::Owned(cli_tool.tool_name()),
                Cow::Owned(cli_tool.description.clone()),
                schema_for_type::<ToolRequest>(),
            );
            let registry_for_route = registry.clone();
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let executor = executor_for_route.clone();
                let tool_owner = tool_owner.clone();
                let tool_name = tool_name.clone();
                let tool_version = tool_version.clone();
                let registry = registry_for_route.clone();
                let session_id = session_id_from_extensions(&ctx.request_context.extensions);
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: ToolRequest = parse_json_object(arguments)?;
                    let (timeout_seconds, max_tokens) =
                        match parse_caps(&req.timeout, req.max_tokens) {
                            Ok(v) => v,
                            Err(msg) => {
                                let item = synthetic_error(msg).into_mcp();
                                return Ok(CallToolResult::success(format_items(vec![item])));
                            }
                        };
                    let transform = build_transform(req.jq, req.python);
                    let request = tools::run::Request {
                        path_type: tools::run::Path::ToolsRun,
                        owner: tool_owner,
                        name: tool_name,
                        version: tool_version,
                        args: req.args,
                        base: RequestBase {
                            jq: None,
                            python: None,
                            timeout_seconds: Some(timeout_seconds),
                            max_tokens: Some(max_tokens),
                        },
                    };
                    let state = match session_id {
                        Some(sid) => registry.get(&sid.into()).await,
                        None => None,
                    };
                    let blocks = dispatch_tools_run(
                        &*executor,
                        request,
                        transform,
                        state.as_deref().map(|s| &s.args),
                    )
                    .await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        Self {
            tool_router,
            executor,
            registry,
            tools_by_tool_name,
            plugins_by_tool_name,
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
    agent_arguments: Option<&AgentArguments>,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    match transform {
        None => {
            let stream =
                match objectiveai_sdk::cli::command::execute(executor, request, agent_arguments)
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
                agent_arguments,
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

async fn dispatch_plugins_run<E>(
    executor: &E,
    request: plugins::run::Request,
    transform: Option<Transform>,
    agent_arguments: Option<&AgentArguments>,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    match transform {
        None => {
            let stream = match plugins::run::execute(executor, request, agent_arguments).await {
                Ok(s) => s,
                Err(e) => {
                    return format_items(vec![convert::<plugins::run::ResponseItem, _>(Err(e))]);
                }
            };
            let items: Vec<McpResponseItem> =
                stream.map(convert::<plugins::run::ResponseItem, _>).collect().await;
            format_items(items)
        }
        Some(t) => {
            let stream =
                match plugins::run::execute_transform(executor, request, t, agent_arguments).await {
                    Ok(s) => s,
                    Err(e) => return format_items(vec![convert_value(Err(e))]),
                };
            let items: Vec<McpResponseItem> = stream.map(convert_value).collect().await;
            format_items(items)
        }
    }
}

async fn dispatch_tools_run<E>(
    executor: &E,
    request: tools::run::Request,
    transform: Option<Transform>,
    agent_arguments: Option<&AgentArguments>,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    match transform {
        None => {
            let stream = match tools::run::execute(executor, request, agent_arguments).await {
                Ok(s) => s,
                Err(e) => {
                    return format_items(vec![convert::<tools::run::ResponseItem, _>(Err(e))]);
                }
            };
            let items: Vec<McpResponseItem> =
                stream.map(convert::<tools::run::ResponseItem, _>).collect().await;
            format_items(items)
        }
        Some(t) => {
            let stream =
                match tools::run::execute_transform(executor, request, t, agent_arguments).await {
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
// custom bit: it filters the macro-default's `self.tool_router
// .list_all()` by the per-session `ClientObjectiveaiMcpSessionFilter`
// stamped by `header_session_manager::extract_mcp_filter`.
//
// Classification order in `list_tools`:
//   1. `ObjectiveAI` ⇒ gated on `filter.root`.
//   2. Tool name in `tools_by_tool_name` ⇒ filtered by
//      `filter.tools` (None ⇒ allow; Some ⇒ membership check).
//   3. Tool name in `plugins_by_tool_name` ⇒ same with
//      `filter.plugins`.
//   4. Anything else ⇒ allow (defensive; the existing route loop
//      registers nothing outside those three categories).
//
// No filter recorded for the session (no header parser ran, e.g.
// GET-only flow) ⇒ behave as `root=true, tools=None, plugins=None`
// — every tool advertised. Mirrors the user-spelled "absent ⇒
// default" semantics for each field.
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
        let tool_filter = state.as_deref().and_then(|s| s.mcp_tools.as_deref());
        let plugin_filter = state.as_deref().and_then(|s| s.mcp_plugins.as_deref());

        let tools = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|t| {
                if t.name.as_ref() == "ObjectiveAI" {
                    return root;
                }
                if let Some(entry) = self.tools_by_tool_name.get(t.name.as_ref()) {
                    return tool_filter.map_or(true, |f| f.contains(entry));
                }
                if let Some(entry) = self.plugins_by_tool_name.get(t.name.as_ref()) {
                    return plugin_filter.map_or(true, |f| f.contains(entry));
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
