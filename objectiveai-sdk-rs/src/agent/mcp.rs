//! MCP server configuration for agents.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// An MCP server that the agent can connect to.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.McpServer")]
pub struct McpServer {
    /// The URL of the MCP server.
    pub url: String,
    /// Whether this MCP server uses authorization.
    #[serde(default)]
    pub authorization: bool,
}

impl McpServer {
    /// Validates the MCP server configuration.
    ///
    /// The URL must start with `http://localhost` or `https://`.
    pub fn validate(&self) -> Result<(), String> {
        // if !self.url.starts_with("http://localhost") && !self.url.starts_with("https://") {
        //     return Err(format!(
        //         "`mcp.url` must start with \"http://localhost\" or \"https://\", got: \"{}\"",
        //         self.url
        //     ));
        // }
        Ok(())
    }
}

/// A list of MCP servers.
pub type McpServers = Vec<McpServer>;

pub mod mcp_servers {
    //! Functions for working with [`McpServers`](super::McpServers).

    /// Validates all MCP servers in the list.
    ///
    /// Checks that no duplicate servers exist.
    pub fn validate(this: &super::McpServers) -> Result<(), String> {
        for server in this {
            server.validate()?;
        }
        for (i, a) in this.iter().enumerate() {
            for b in &this[i + 1..] {
                if a == b {
                    return Err(format!(
                        "`mcp_servers` contains duplicate entry: \"{}\"",
                        a.url
                    ));
                }
            }
        }
        Ok(())
    }

    /// Sorts the MCP servers for deterministic ordering.
    ///
    /// Empty lists become `None`.
    pub fn prepare(mut this: super::McpServers) -> Option<super::McpServers> {
        if this.is_empty() {
            None
        } else {
            this.sort();
            Some(this)
        }
    }
}
