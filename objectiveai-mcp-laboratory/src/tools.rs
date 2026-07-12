use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Extension,
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
    /// MCP server name reported in `get_info`: `oail-<RAW id>` — the
    /// raw laboratory id parsed out of the COMPOSITE
    /// `OBJECTIVEAI_LABORATORY_ID` (`{machine}/{state}/{id}`, state
    /// and id base62-encoded), or `oail` when run standalone. The
    /// server name feeds the proxy's tool-name prefix, which cannot
    /// carry the composite (provider name limits).
    server_name: String,
    /// The full composite laboratory id, verbatim from the env —
    /// surfaced via the server `instructions` so the assistant can
    /// quote it (e.g. as `laboratory_transfer` source/destination).
    /// `None` standalone or on a legacy (raw-id) env value.
    composite_id: Option<String>,
}

#[tool_router]
impl ObjectiveAiMcpLaboratory {
    pub fn new(laboratory_id: Option<String>, default_cwd: std::path::PathBuf) -> Self {
        let (server_name, composite_id) = match laboratory_id {
            Some(value) => match crate::composite::parse_composite_laboratory_id(&value) {
                Some((_machine, _state, raw_id)) => {
                    (format!("oail-{raw_id}"), Some(value))
                }
                // Tolerant legacy fallback: a container created before
                // the composite era carries the raw id verbatim —
                // exactly the old behavior, no composite to surface.
                None => (format!("oail-{value}"), None),
            },
            None => ("oail".to_string(), None),
        };
        Self {
            tool_router: Self::tool_router(),
            shell_state: crate::bash::ShellState::new(default_cwd),
            server_name,
            composite_id,
        }
    }

    /// Initialize session state (shell snapshot, etc.).
    /// Should be called once after construction.
    pub async fn init(&self) {
        self.shell_state.init_snapshot().await;
    }

    #[tool(name = "Bash", description = "Executes a given bash command and returns its output.")]
    async fn bash(
        &self,
        Parameters(req): Parameters<BashRequest>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Content {
        // The conduit forwards `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` on every
        // tools/call (1:1 with the agent instance). cwd/env are kept per-AIH so
        // concurrent agents sharing this lab don't trample each other; a
        // missing/empty header (standalone runs) routes to the `""` bucket.
        // `HeaderMap::get` is case-insensitive.
        let aih = parts
            .headers
            .get("x-objectiveai-agent-instance-hierarchy")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        match crate::bash::execute_bash(&self.shell_state, aih, &req.command, req.timeout).await {
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
        server_info.name = self.server_name.clone().into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        // The assistant's in-band way to learn this laboratory's FULL
        // id (machine/state/id — ids are only unique per (machine,
        // state)): quote it verbatim, e.g. for `laboratory_transfer`.
        info.instructions = self.composite_id.as_ref().map(|composite| {
            format!(
                "This laboratory's id is `{composite}`. Use it verbatim wherever \
                 a laboratory id is required (e.g. `laboratory_transfer` \
                 source/destination)."
            )
        });
        info
    }
}
