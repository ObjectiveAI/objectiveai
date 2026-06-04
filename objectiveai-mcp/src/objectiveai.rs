use std::borrow::Cow;
use std::sync::Arc;

use futures::FutureExt;
use futures::StreamExt;
use objectiveai_sdk::cli::Error as CliError;
use objectiveai_sdk::cli::ErrorType as CliErrorType;
use objectiveai_sdk::cli::Level as CliLevel;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::CommandResponse;
use objectiveai_sdk::cli::command::McpResponseItem;
use objectiveai_sdk::cli::command::Request;
use objectiveai_sdk::cli::command::ResponseItem;
use objectiveai_sdk::cli::command::parse_request;
use objectiveai_sdk::cli::command::plugins;
use objectiveai_sdk::cli::command::tools;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::{parse_json_object, schema_for_type},
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

#[derive(Debug)]
pub struct ObjectiveAiMcpCli<E> {
    pub tool_router: ToolRouter<Self>,
    pub executor: Arc<E>,
}

impl<E> Clone for ObjectiveAiMcpCli<E> {
    fn clone(&self) -> Self {
        Self {
            tool_router: self.tool_router.clone(),
            executor: self.executor.clone(),
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
    ) -> Self {
        let mut tool_router = Self::tool_router();
        for plugin in plugins_list {
            if plugin.name == "ObjectiveAI" {
                continue;
            }
            let plugin_name = plugin.name.clone();
            let executor_for_route = executor.clone();
            let tool = Tool::new(
                Cow::Owned(plugin.name.clone()),
                Cow::Owned(plugin.description.clone()),
                schema_for_type::<PluginRequest>(),
            );
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let executor = executor_for_route.clone();
                let plugin_name = plugin_name.clone();
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: PluginRequest = parse_json_object(arguments)?;
                    let request = plugins::run::Request {
                        name: plugin_name,
                        args: req.args,
                        jq: None,
                    };
                    let blocks = dispatch_plugins_run(&*executor, request).await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        for cli_tool in tools_list {
            if cli_tool.name == "ObjectiveAI" {
                continue;
            }
            let tool_name = cli_tool.name.clone();
            let executor_for_route = executor.clone();
            let tool = Tool::new(
                Cow::Owned(cli_tool.name.clone()),
                Cow::Owned(cli_tool.description.clone()),
                schema_for_type::<ToolRequest>(),
            );
            tool_router.add_route(ToolRoute::new_dyn(tool, move |ctx| {
                let executor = executor_for_route.clone();
                let tool_name = tool_name.clone();
                async move {
                    let arguments = ctx.arguments.unwrap_or_default();
                    let req: ToolRequest = parse_json_object(arguments)?;
                    let request = tools::run::Request {
                        name: tool_name,
                        args: req.args,
                        jq: None,
                    };
                    let blocks = dispatch_tools_run(&*executor, request).await;
                    Ok(CallToolResult::success(blocks))
                }
                .boxed()
            }));
        }
        Self {
            tool_router,
            executor,
        }
    }

    #[tool(name = "ObjectiveAI", description = "Run an ObjectiveAI command.")]
    async fn objectiveai(
        &self,
        Parameters(req): Parameters<ObjectiveAiRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let request = match parse_request(&req.command) {
            Ok(r) => r,
            Err(e) => {
                let item = synthetic_error(e.to_string()).into_mcp();
                return Ok(CallToolResult::success(format_items(vec![item])));
            }
        };
        let blocks = dispatch_root(&*self.executor, request).await;
        Ok(CallToolResult::success(blocks))
    }
}

async fn dispatch_root<E>(executor: &E, request: Request) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    let stream = match objectiveai_sdk::cli::command::execute(executor, request).await {
        Ok(s) => s,
        Err(e) => return format_items(vec![convert::<ResponseItem>(Err(e))]),
    };
    let items: Vec<McpResponseItem> = stream.map(convert::<ResponseItem>).collect().await;
    format_items(items)
}

async fn dispatch_plugins_run<E>(
    executor: &E,
    request: plugins::run::Request,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    let stream = match plugins::run::execute(executor, request).await {
        Ok(s) => s,
        Err(e) => return format_items(vec![convert::<plugins::run::ResponseItem>(Err(e))]),
    };
    let items: Vec<McpResponseItem> =
        stream.map(convert::<plugins::run::ResponseItem>).collect().await;
    format_items(items)
}

async fn dispatch_tools_run<E>(
    executor: &E,
    request: tools::run::Request,
) -> Vec<rmcp::model::Content>
where
    E: CommandExecutor,
    E::Error: std::fmt::Display,
{
    let stream = match tools::run::execute(executor, request).await {
        Ok(s) => s,
        Err(e) => return format_items(vec![convert::<tools::run::ResponseItem>(Err(e))]),
    };
    let items: Vec<McpResponseItem> =
        stream.map(convert::<tools::run::ResponseItem>).collect().await;
    format_items(items)
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

#[tool_handler]
impl<E> ServerHandler for ObjectiveAiMcpCli<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
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
