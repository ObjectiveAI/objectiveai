use std::sync::Arc;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ObjectiveAiRequest {
    #[schemars(description = "The command arguments to pass to the ObjectiveAI CLI (e.g. [\"agents\", \"list\"] or [\"functions\", \"executions\", \"create\", \"--help\"])")]
    pub command: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectiveAiMcpCli {
    pub tool_router: ToolRouter<Self>,
    pub cli_config: Arc<objectiveai_cli::Config>,
}

#[tool_router]
impl ObjectiveAiMcpCli {
    pub fn new(cli_config: Arc<objectiveai_cli::Config>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            cli_config,
        }
    }

    #[tool(
        name = "ObjectiveAI CLI",
        description = "Run an ObjectiveAI CLI command."
    )]
    async fn objectiveai(
        &self,
        Parameters(req): Parameters<ObjectiveAiRequest>,
    ) -> String {
        let args: Vec<String> = std::iter::once("objectiveai".to_string())
            .chain(req.command)
            .collect();

        match objectiveai_cli::run(args, &self.cli_config).await {
            Ok(output) => output,
            Err(e) => format!("error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for ObjectiveAiMcpCli {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Run an ObjectiveAI CLI command.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
