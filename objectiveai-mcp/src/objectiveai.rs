use std::borrow::Cow;
use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use http::request::Parts;
use objectiveai_cli::filesystem::plugins::ManifestWithNameAndSource as PluginManifest;
use objectiveai_cli::filesystem::tools::ManifestWithNameAndSource as ToolManifest;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::{Extension, parse_json_object, schema_for_type},
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    },
    schemars, tool, tool_handler, tool_router,
};

use crate::format::format_items;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ObjectiveAiRequest {
    #[schemars(
        description = "The command arguments to pass to the ObjectiveAI CLI (e.g. [\"agents\", \"list\"] or [\"functions\", \"executions\", \"create\", \"--help\"])"
    )]
    pub command: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PluginRequest {
    #[schemars(
        description = "Args forwarded to the plugin's argv (prefixed automatically with `plugins run <name>` when invoking the CLI)."
    )]
    pub args: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ToolRequest {
    #[schemars(
        description = "Args forwarded to the tool's argv (prefixed automatically with `tools run <name>` when invoking the CLI)."
    )]
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectiveAiMcpCli {
    pub tool_router: ToolRouter<Self>,
    pub cli_config: Arc<objectiveai_cli::Config>,
}

#[tool_router]
impl ObjectiveAiMcpCli {
    /// Build a handler with one dynamic tool per discovered CLI plugin
    /// and CLI tool, in addition to the static `ObjectiveAI` catch-all.
    /// Plugins and tools are listed once at server startup (see
    /// `run::setup`); this constructor is not re-invoked when either
    /// is added later, so hot reload is intentionally out of scope.
    ///
    /// Name collisions: if a CLI plugin and a CLI tool happen to share
    /// a name (or with `ObjectiveAI`), the plugin registers first and
    /// the tool's `add_route` overwrites it (last-writer-wins) — but
    /// `ObjectiveAI` itself is always skipped on both sides so the
    /// built-in catch-all is never shadowed.
    pub fn with_plugins_and_tools(
        cli_config: Arc<objectiveai_cli::Config>,
        plugins: Vec<PluginManifest>,
        tools: Vec<ToolManifest>,
    ) -> Self {
        let mut tool_router = Self::tool_router();
        for plugin in plugins {
            if plugin.name == "ObjectiveAI" {
                continue;
            }
            let plugin_name = plugin.name.clone();
            let cli_config_for_route = cli_config.clone();
            let tool = Tool::new(
                Cow::Owned(plugin.name.clone()),
                Cow::Owned(plugin.manifest.description.clone()),
                schema_for_type::<PluginRequest>(),
            );
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let cli_config = cli_config_for_route.clone();
                let plugin_name = plugin_name.clone();
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: PluginRequest = parse_json_object(arguments)?;
                    let parts = ctx
                        .request_context
                        .extensions
                        .get::<Parts>()
                        .cloned()
                        .unwrap_or_else(|| http::Request::new(()).into_parts().0);
                    let args: Vec<String> = [
                        "objectiveai".to_string(),
                        "plugins".to_string(),
                        "run".to_string(),
                        plugin_name,
                    ]
                    .into_iter()
                    .chain(req.args.into_iter())
                    .collect();
                    let blocks = run_cli_and_collect(&cli_config, &parts, args).await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        for cli_tool in tools {
            if cli_tool.name == "ObjectiveAI" {
                continue;
            }
            let tool_name = cli_tool.name.clone();
            let cli_config_for_route = cli_config.clone();
            let tool = Tool::new(
                Cow::Owned(cli_tool.name.clone()),
                Cow::Owned(cli_tool.manifest.description.clone()),
                schema_for_type::<ToolRequest>(),
            );
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let cli_config = cli_config_for_route.clone();
                let tool_name = tool_name.clone();
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: ToolRequest = parse_json_object(arguments)?;
                    let parts = ctx
                        .request_context
                        .extensions
                        .get::<Parts>()
                        .cloned()
                        .unwrap_or_else(|| http::Request::new(()).into_parts().0);
                    let args: Vec<String> = [
                        "objectiveai".to_string(),
                        "tools".to_string(),
                        "run".to_string(),
                        tool_name.clone(),
                    ]
                    .into_iter()
                    .chain(req.args.into_iter())
                    .collect();
                    let blocks = run_cli_and_collect(&cli_config, &parts, args).await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        Self {
            tool_router,
            cli_config,
        }
    }

    #[tool(name = "ObjectiveAI", description = "Run an ObjectiveAI command.")]
    async fn objectiveai(
        &self,
        Parameters(req): Parameters<ObjectiveAiRequest>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let args: Vec<String> = std::iter::once("objectiveai".to_string())
            .chain(req.command)
            .collect();
        // Catch-all dispatches arbitrary `objectiveai …` commands.
        // The formatter has a single rendering mode now; see
        // `crate::format` for the full dispatch table.
        let blocks = run_cli_and_collect(&self.cli_config, &parts, args).await;
        Ok(CallToolResult::success(blocks))
    }
}

/// Run the ObjectiveAI CLI in-process with `args`, drain its typed
/// `RunItem` stream into a `Vec`, and format the result into the MCP
/// tool response `Vec<Content>`. Applies the per-request
/// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` header override (clones
/// the server-wide `cli_config` so concurrent requests stay
/// independent) by building a fresh [`objectiveai_cli::context::Context`]
/// for every call — the agent_instance_hierarchy stamp is baked into
/// the `HttpClient` at construction time, so per-request HTTP-header
/// overrides require a fresh client anyway.
///
/// See [`crate::format`] for the dispatch table.
async fn run_cli_and_collect(
    cli_config: &Arc<objectiveai_cli::Config>,
    parts: &Parts,
    args: Vec<String>,
) -> Vec<rmcp::model::Content> {
    // Per-request: stamp agent_instance_hierarchy + mcp_session_id from headers so
    // every cli invocation (and every tool subprocess the cli spawns
    // transitively) sees the values relevant to *this* request.
    // Clone-then-mutate-then-Arc so concurrent requests see
    // independent values.
    //
    // The MCP session id is read from `Mcp-Session-Id` (the rmcp
    // transport's standard header). When the upstream MCP client
    // doesn't actually manage sessions, fall back to the per-agent
    // lineage id from `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` — stable per agent and
    // unique across agents, which is exactly what session-keyed tool
    // state (e.g. per-session counters) wants.
    let mut cfg = (**cli_config).clone();
    let header_agent_instance_hierarchy = parts
        .headers
        .get("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY")
        .and_then(|h| h.to_str().ok());
    let header_agent_id = parts
        .headers
        .get("X-OBJECTIVEAI-AGENT-ID")
        .and_then(|h| h.to_str().ok());
    // Always populate agent_instance_hierarchy for MCP-routed calls. When the upstream
    // MCP client didn't send the header, default to "MCP" so
    // `agents me` (and any other code reading the field) reports
    // the call's actual origin instead of inheriting the server-wide
    // default ("CLI") — which would be misleading.
    cfg.agent_instance_hierarchy = header_agent_instance_hierarchy.unwrap_or("mcp").to_string();
    // Override agent_id only when the upstream supplied it;
    // otherwise keep whatever the server was configured with. Empty
    // string is treated as "absent" so a blank header doesn't blank
    // out the inherited base.
    if let Some(base) = header_agent_id.filter(|s| !s.is_empty()) {
        cfg.agent_id = Some(base.to_string());
    }
    let header_session_id = parts
        .headers
        .get(objectiveai_sdk::mcp::MCP_SESSION_ID_HEADER)
        .and_then(|h| h.to_str().ok());
    // session_id falls back to the *header-provided* agent_instance_hierarchy only —
    // not the "MCP" default — so the per-session tool state key stays
    // meaningful when the client doesn't actually manage sessions.
    cfg.mcp_session_id = match header_session_id {
        Some(s) => Some(s.to_string()),
        None => header_agent_instance_hierarchy.map(str::to_string),
    };

    let ctx = match objectiveai_cli::context::Context::new(cfg).await {
        Ok(c) => c,
        Err(e) => return format_items(&[Err(e)]),
    };
    let stream = match objectiveai_cli::run(args, Some(ctx)).await {
        Ok(s) => s,
        Err(e) => return format_items(&[Err(e)]),
    };
    let items: Vec<Result<objectiveai_cli::RunItem, objectiveai_cli::error::Error>> =
        stream.collect().await;
    format_items(&items)
}

#[tool_handler]
impl ServerHandler for ObjectiveAiMcpCli {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "oai".into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}
