//! Codex's `config.toml`, rendered from the agent.
//!
//! Every key written here is one the agent names or one the harness
//! always sets, and nothing else: the model and its three knobs, the
//! web-search mode (`disabled` when the agent says nothing), the
//! reasoning shown, the one MCP server — the proxy's, at the
//! container SDK's `mcp_url()` — and, when the agent names a
//! provider, the `diverge` entry and its selection. Sandbox and
//! approval are not here: they ride the argv as
//! `--dangerously-bypass-approvals-and-sandbox`, so there is one
//! spelling of that decision.
//!
//! Written before every run: the agent never changes, but the file
//! is the run's, and a mounted home may not carry one.

use std::collections::BTreeMap;
use std::io;

use serde::Serialize;

use crate::agent::{Agent, Effort, Summary, Verbosity, WebSearch};
use crate::auth::CODEX_HOME;

/// Where the file goes.
pub const CONFIG_FILE: &str = "/root/.codex/config.toml";

/// The name of the one MCP server, and of the provider entry.
pub const DIVERGE: &str = "diverge";

/// The file's shape: the keys, as Codex spells them.
#[derive(Debug, Serialize)]
struct Config<'a> {
    model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_reasoning_effort: Option<Effort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_reasoning_summary: Option<Summary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_verbosity: Option<Verbosity>,
    web_search: WebSearch,
    hide_agent_reasoning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_provider: Option<&'static str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    model_providers: BTreeMap<&'static str, Provider<'a>>,
    mcp_servers: BTreeMap<&'static str, McpServer>,
}

/// One `model_providers` entry.
#[derive(Debug, Serialize)]
struct Provider<'a> {
    name: &'static str,
    base_url: &'a str,
    env_key: &'static str,
    wire_api: &'static str,
}

/// One `mcp_servers` entry: a streamable HTTP server, by its URL.
#[derive(Debug, Serialize)]
struct McpServer {
    url: String,
}

/// The file's contents for the agent.
pub fn render(agent: &Agent) -> Result<String, toml::ser::Error> {
    let mut model_providers = BTreeMap::new();
    let mut model_provider = None;
    if let Some(provider) = &agent.provider {
        model_providers.insert(
            DIVERGE,
            Provider {
                name: DIVERGE,
                base_url: &provider.base_url,
                env_key: crate::auth::API_KEY,
                wire_api: "responses",
            },
        );
        model_provider = Some(DIVERGE);
    }
    let mut mcp_servers = BTreeMap::new();
    mcp_servers.insert(
        DIVERGE,
        McpServer {
            url: diverge_container_proxy_sdk::mcp_url(),
        },
    );
    toml::to_string(&Config {
        model: &agent.model,
        model_reasoning_effort: agent.effort,
        model_reasoning_summary: agent.reasoning_summary,
        model_verbosity: agent.verbosity,
        web_search: agent.web_search.unwrap_or(WebSearch::Disabled),
        hide_agent_reasoning: false,
        model_provider,
        model_providers,
        mcp_servers,
    })
}

/// Write the contents at [`CONFIG_FILE`], the home made as needed.
pub async fn write(contents: &str) -> io::Result<()> {
    tokio::fs::create_dir_all(CODEX_HOME).await?;
    tokio::fs::write(CONFIG_FILE, contents).await
}
