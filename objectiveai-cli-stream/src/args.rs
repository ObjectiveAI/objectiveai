//! Shared clap argument groups every endpoint subcommand flattens.
//!
//! Required vs optional mirrors the existing `objectiveai-cli`'s
//! env-var-or-config resolvers at
//! `objectiveai-cli/src/api/{client,conduit}.rs`. `--api-address`
//! is the only thing strictly required for a streaming endpoint;
//! every other header / MCP / pipe arg is optional with the same
//! defaults the CLI uses today.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;

/// HTTP-client construction args. Every field maps 1:1 onto the
/// corresponding env var the existing `objectiveai-cli`'s
/// `build_http_client` resolves.
#[derive(Args, Debug, Clone)]
pub struct HttpArgs {
    /// Full base URL of the ObjectiveAI API to dial
    /// (e.g. `http://127.0.0.1:8080`). Maps to `OBJECTIVEAI_ADDRESS`.
    #[arg(long, global = true)]
    pub api_address: Option<String>,

    /// `Authorization` header. Maps to `OBJECTIVEAI_AUTHORIZATION`.
    #[arg(long, global = true)]
    pub objectiveai_authorization: Option<String>,

    /// `User-Agent` header. Maps to `USER_AGENT`.
    #[arg(long, global = true)]
    pub user_agent: Option<String>,

    /// `X-Title` header. Maps to `X_TITLE`.
    #[arg(long, global = true)]
    pub x_title: Option<String>,

    /// `HTTP-Referer` header. Maps to `HTTP_REFERER`.
    #[arg(long, global = true)]
    pub http_referer: Option<String>,

    /// `X-GITHUB-AUTHORIZATION` header. Maps to `GITHUB_AUTHORIZATION`.
    #[arg(long, global = true)]
    pub github_authorization: Option<String>,

    /// `X-OPENROUTER-AUTHORIZATION` header. Maps to `OPENROUTER_AUTHORIZATION`.
    #[arg(long, global = true)]
    pub openrouter_authorization: Option<String>,

    /// `X-MCP-AUTHORIZATION` header — JSON-encoded `HashMap<String,String>`,
    /// e.g. `'{"server.example.com":"bearer-token"}'`. Maps to `MCP_AUTHORIZATION`.
    #[arg(long, global = true)]
    pub mcp_authorization: Option<String>,

    /// `X-VIEWER-SIGNATURE` header. Maps to `VIEWER_SIGNATURE`.
    #[arg(long, global = true)]
    pub viewer_signature: Option<String>,

    /// Viewer base URL (full, e.g. `http://127.0.0.1:8081`).
    /// Maps to `VIEWER_ADDRESS`.
    #[arg(long, global = true)]
    pub viewer_address: Option<String>,

    /// `X-COMMIT-AUTHOR-NAME` header. Maps to `COMMIT_AUTHOR_NAME`.
    #[arg(long, global = true)]
    pub commit_author_name: Option<String>,

    /// `X-COMMIT-AUTHOR-EMAIL` header. Maps to `COMMIT_AUTHOR_EMAIL`.
    #[arg(long, global = true)]
    pub commit_author_email: Option<String>,

    /// `X-OBJECTIVEAI-AGENT-ID` header. Maps to `OBJECTIVEAI_AGENT_ID`.
    #[arg(long, global = true)]
    pub objectiveai_agent_id: Option<String>,
}

impl HttpArgs {
    /// Build the SDK `HttpClient` from the parsed args. Mirrors
    /// `objectiveai-cli/src/api/client.rs::build_http_client` 1:1
    /// except every value comes from clap instead of env-var-or-config.
    ///
    /// `--api-address` is required at the endpoint level — when the
    /// subcommand actually opens a stream it must be set. The arg
    /// itself is declared `Option<String>` so that `--help` works
    /// at the top level without users having to also pass an
    /// address; the endpoint subcommand validates presence before
    /// calling this.
    pub fn build_http_client(&self) -> Result<objectiveai_sdk::HttpClient, String> {
        let x_mcp_authorization: Option<HashMap<String, String>> = self
            .mcp_authorization
            .as_deref()
            .map(|s| {
                serde_json::from_str(s).map_err(|e| {
                    format!(
                        "--mcp-authorization is not valid JSON for HashMap<String,String>: {e}"
                    )
                })
            })
            .transpose()?;

        let address = self.api_address.clone().ok_or_else(|| {
            "--api-address is required for streaming endpoints".to_string()
        })?;

        Ok(objectiveai_sdk::HttpClient::new(
            reqwest::Client::new(),
            Some(address),
            self.objectiveai_authorization.clone(),
            self.user_agent.clone(),
            self.x_title.clone(),
            self.http_referer.clone(),
            self.github_authorization.clone(),
            self.openrouter_authorization.clone(),
            x_mcp_authorization,
            self.viewer_signature.clone(),
            self.viewer_address.clone(),
            self.commit_author_name.clone(),
            self.commit_author_email.clone(),
            self.objectiveai_agent_id.clone(),
        ))
    }
}

/// MCP-conduit + pipe args.
#[derive(Args, Debug, Clone)]
pub struct PipeArgs {
    /// Root directory under which per-agent pipes are bound. Pipes
    /// live at `${config_base_dir}/pipes/<agent_id>` on POSIX (slashes
    /// in the agent id become subdirectories; the final segment is
    /// the socket file's name). On Windows the same logical name
    /// goes into the flat `\\.\pipe\` namespace with `/` re-encoded
    /// as `_`. Maps loosely to the existing CLI's `CONFIG_BASE_DIR`.
    #[arg(long, global = true)]
    pub config_base_dir: Option<PathBuf>,

    /// Full base URL of the local `objectiveai-mcp` server the
    /// conduit dials when the API forwards an inbound `server_request`.
    /// When unset, every inbound MCP request 501s — matches the
    /// existing CLI's behavior when neither `OBJECTIVEAI_MCP_ADDRESS`
    /// nor on-disk config supplies one.
    #[arg(long, global = true)]
    pub mcp_address: Option<String>,
}

impl PipeArgs {
    /// Build the MCP conduit handler. `--mcp-address` of `None`
    /// makes the handler 501 every inbound request.
    pub fn build_conduit(&self) -> crate::conduit::ConduitMcpHandler {
        crate::conduit::ConduitMcpHandler::new(self.mcp_address.clone())
    }

    /// `${config_base_dir}/pipes` — the root every per-agent pipe
    /// is bound under. Required at the endpoint level; same Option
    /// declaration rationale as `HttpArgs::api_address`.
    pub fn pipes_root(&self) -> Result<PathBuf, String> {
        let base = self
            .config_base_dir
            .as_deref()
            .ok_or_else(|| "--config-base-dir is required for streaming endpoints".to_string())?;
        Ok(base.join("pipes"))
    }
}

/// JSON-body input. Mirrors the JSON variants of
/// `objectiveai-cli/src/api/body.rs::BodySource` (the python flavors
/// stay in the original CLI since they require its `python` module).
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct BodySource {
    /// Inline JSON body.
    #[arg(long)]
    pub body_inline: Option<String>,

    /// Path to a JSON body file.
    #[arg(long)]
    pub body_file: Option<PathBuf>,
}

impl BodySource {
    /// Parse the configured body into `T`.
    pub fn resolve<T: serde::de::DeserializeOwned>(self) -> Result<T, String> {
        if let Some(inline) = self.body_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de).map_err(|e| {
                format!("--body-inline parse error at `{}`: {}", e.path(), e.inner())
            });
        }
        if let Some(path) = self.body_file {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| format!("--body-file read error ({}): {e}", path.display()))?;
            let mut de = serde_json::Deserializer::from_str(&contents);
            return serde_path_to_error::deserialize(&mut de).map_err(|e| {
                format!(
                    "--body-file parse error ({}) at `{}`: {}",
                    path.display(),
                    e.path(),
                    e.inner()
                )
            });
        }
        unreachable!("clap group ensures one is set")
    }
}
