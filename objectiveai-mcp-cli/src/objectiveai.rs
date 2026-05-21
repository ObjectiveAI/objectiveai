use std::sync::Arc;
use http::request::Parts;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Extension,
    handler::server::wrapper::Parameters,
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
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
        Extension(parts): Extension<Parts>,
    ) -> String {
        let args: Vec<String> = std::iter::once("objectiveai".to_string())
            .chain(req.command)
            .collect();

        // Per-request: if the caller sent X-OBJECTIVEAI-AGENT-ID,
        // override the server-wide cli_config.agent_id for this
        // invocation only. Clone-then-mutate-then-Arc so concurrent
        // requests see independent values.
        let cli_config: Arc<objectiveai_cli::Config> = match parts
            .headers
            .get("X-OBJECTIVEAI-AGENT-ID")
            .and_then(|h| h.to_str().ok())
        {
            Some(agent_id) => {
                let mut cfg = (*self.cli_config).clone();
                cfg.agent_id = Some(agent_id.to_string());
                Arc::new(cfg)
            }
            None => self.cli_config.clone(),
        };

        let collected = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        // Per-request handle: stamp the agent_id from the per-request
        // cli_config so every notification/error the cli emits during
        // this Run carries `X-OBJECTIVEAI-AGENT-ID`. The Collect
        // destination mirrors the same agent_id into the in-memory
        // Vec the runner reassembles below.
        let handle = objectiveai_sdk::cli::output::Handle {
            destination: objectiveai_sdk::cli::output::HandleDestination::Collect(
                collected.clone(),
            ),
            agent_id: cli_config.agent_id.clone(),
        };
        let code = objectiveai_cli::run(args, &cli_config, handle).await;

        let outputs = collected.lock().await;
        let mut buf = String::new();
        for output in outputs.iter() {
            match serde_json::to_string(output) {
                Ok(line) => buf.push_str(&line),
                Err(e) => buf.push_str(&format!("error serializing output: {e}")),
            }
            buf.push('\n');
        }
        if code != 0 {
            buf.push_str(&format!("exit code: {code}\n"));
        }
        buf
    }
}

#[tool_handler]
impl ServerHandler for ObjectiveAiMcpCli {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "oaicli".into(),
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
