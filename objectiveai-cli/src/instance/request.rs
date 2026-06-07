//! Typed request blob the parent `objectiveai-cli` process writes to
//! the inherited handshake pipe, and the child `instance` entrypoint
//! reads + dispatches on. Replaces the previous clap-based arg
//! surface (`HttpArgs`, `PipeArgs`, `BodySource`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use serde::{Deserialize, Serialize};

/// One JSON object delivered over the parent → child handshake pipe
/// after the magic header line. Wire form is plain serde JSON; the
/// child deserializes via `serde_path_to_error` for good diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRequest {
    pub http: HttpConfig,
    pub pipes: PipeConfig,
    pub endpoint: InstanceEndpoint,
}

/// HTTP-client configuration. Plain-struct equivalent of the previous
/// `HttpArgs` clap surface. Every field maps 1:1 onto the corresponding
/// env var the regular `objectiveai-cli`'s `build_http_client` resolves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub api_address: Option<String>,
    pub objectiveai_authorization: Option<String>,
    pub user_agent: Option<String>,
    pub x_title: Option<String>,
    pub http_referer: Option<String>,
    pub github_authorization: Option<String>,
    pub openrouter_authorization: Option<String>,
    pub mcp_authorization: Option<HashMap<String, String>>,
    pub viewer_signature: Option<String>,
    pub viewer_address: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub objectiveai_agent_instance_hierarchy: String,
    pub mcp_session_id: Option<String>,
}

impl HttpConfig {
    /// Build the SDK `HttpClient`. Mirrors the regular CLI's
    /// `build_http_client` 1:1 except every value is already resolved.
    pub fn build_http_client(&self) -> Result<objectiveai_sdk::HttpClient, String> {
        let address = self
            .api_address
            .clone()
            .ok_or_else(|| "api_address is required for streaming endpoints".to_string())?;
        Ok(objectiveai_sdk::HttpClient::new(
            reqwest::Client::new(),
            Some(address),
            self.objectiveai_authorization.clone(),
            self.user_agent.clone(),
            self.x_title.clone(),
            self.http_referer.clone(),
            self.github_authorization.clone(),
            self.openrouter_authorization.clone(),
            self.mcp_authorization.clone(),
            self.viewer_signature.clone(),
            self.viewer_address.clone(),
            self.commit_author_name.clone(),
            self.commit_author_email.clone(),
            Some(self.objectiveai_agent_instance_hierarchy.clone()),
            self.mcp_session_id.clone(),
        ))
    }
}

/// MCP conduit + per-agent pipe configuration. Plain-struct equivalent
/// of the previous `PipeArgs` clap surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeConfig {
    pub config_base_dir: PathBuf,
}

impl PipeConfig {
    pub fn config_base_dir(&self) -> &Path {
        &self.config_base_dir
    }

    pub fn build_conduit(
        &self,
        ctx: crate::context::Context,
        mcp_server: crate::instance::mcp_server::McpServerHandle,
    ) -> crate::instance::api::conduit::ConduitMcpHandler {
        crate::instance::api::conduit::ConduitMcpHandler::new(mcp_server, ctx)
    }
}

/// Which endpoint to dispatch to. The variant carries the typed
/// SDK params struct so the child doesn't need to re-parse strategy
/// or sub-variant — the consumer leaves already know which leaf they're
/// invoking and stamp the right variant here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "endpoint", content = "params", rename_all = "snake_case")]
pub enum InstanceEndpoint {
    AgentsSpawn(AgentCompletionCreateParams),
    FunctionsExecutionsCreate(FunctionExecutionCreateParams),
    FunctionsInventionsRecursiveCreate(FunctionInventionRecursiveCreateParams),
}
