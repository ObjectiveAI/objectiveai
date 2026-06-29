use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};

// --- Input schemas (matching Claude Code exactly) ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BashRequest {
    #[schemars(description = "The command to execute")]
    command: String,
    #[schemars(description = "Optional timeout in milliseconds (max 600000)")]
    timeout: Option<u64>,
    #[schemars(description = "Clear, concise description of what this command does in active voice")]
    description: Option<String>,
}

// --- Tool server ---

#[derive(Debug, Clone)]
pub struct ObjectiveAiMcpLaboratory {
    pub tool_router: ToolRouter<Self>,
    shell_state: crate::bash::ShellState,
}

#[tool_router]
impl ObjectiveAiMcpLaboratory {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            shell_state: crate::bash::ShellState::new(),
        }
    }

    /// Initialize session state (shell snapshot, etc.).
    /// Should be called once after construction.
    pub async fn init(&self) {
        self.shell_state.init_snapshot().await;
    }

    #[tool(name = "Bash", description = "Executes a given bash command and returns its output.")]
    async fn bash(&self, Parameters(req): Parameters<BashRequest>) -> Content {
        match crate::bash::execute_bash(&self.shell_state, &req.command, req.timeout).await {
            Ok(output) => {
                if output.is_image {
                    if let Some(parsed) = crate::bash::parse_data_uri(&output.stdout) {
                        return Content::image(parsed.data, parsed.media_type);
                    }
                }
                let json = serde_json::to_string_pretty(&output).unwrap_or_default();
                Content::text(json)
            }
            Err(e) => Content::text(e),
        }
    }
}

#[tool_handler]
impl ServerHandler for ObjectiveAiMcpLaboratory {
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`,
        // so build via `Default` + explicit field assignment.
        let mut server_info = Implementation::default();
        server_info.name = "oaifs".into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = None;
        info
    }
}
