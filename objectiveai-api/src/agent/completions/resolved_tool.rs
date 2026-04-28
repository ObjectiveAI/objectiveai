use std::collections::HashMap;
use std::sync::Arc;

/// A resolved tool with its display name and origin.
#[derive(Clone)]
pub enum ResolvedTool {
    /// A response-format tool carrying its description and JSON schema.
    ResponseFormat {
        description: String,
        schema: indexmap::IndexMap<String, serde_json::Value>,
    },
    /// An MCP tool, with its connection and the original tool name on the server.
    Mcp {
        connection: objectiveai::mcp::Connection,
        tool: objectiveai::mcp::tool::Tool,
    },
}

/// Failure to fetch the tool list from the per-agent MCP proxy connection.
#[derive(Debug, thiserror::Error)]
#[error("MCP list_tools error ({url}): {error}")]
pub struct ResolveToolsError {
    pub url: String,
    pub error: Arc<objectiveai::mcp::Error>,
}

/// Resolves tool names from a single MCP connection (the per-agent proxy
/// connection) and an optional response format.
///
/// Owns the `list_tools()` await internally so callers don't have to
/// manage the connection's tool list themselves. Returns the list of
/// resolved tool names (in insertion order) and a map from each name to
/// its [`ResolvedTool`]. The proxy is responsible for any cross-upstream
/// name disambiguation it needs to do — at this layer we no longer
/// manufacture suffix-renamed aliases on top.
pub async fn resolve_tools(
    mcp_connection: Option<&objectiveai::mcp::Connection>,
    response_format: Option<&objectiveai::agent::completions::request::ResponseFormat>,
) -> Result<(Vec<String>, HashMap<String, ResolvedTool>), ResolveToolsError> {
    let mut names = Vec::new();
    let mut map = HashMap::new();

    if let Some(connection) = mcp_connection {
        let tools = connection.list_tools().await.map_err(|error| ResolveToolsError {
            url: connection.url.clone(),
            error,
        })?;
        for tool in tools.iter() {
            names.push(tool.name.clone());
            map.insert(
                tool.name.clone(),
                ResolvedTool::Mcp {
                    connection: connection.clone(),
                    tool: tool.clone(),
                },
            );
        }
    }

    if let Some(objectiveai::agent::completions::request::ResponseFormat::ToolCall {
        name,
        description,
        schema,
        ..
    }) = response_format
    {
        names.push(name.clone());
        map.insert(
            name.clone(),
            ResolvedTool::ResponseFormat {
                description: description.clone(),
                schema: schema.clone(),
            },
        );
    }

    Ok((names, map))
}

#[cfg(test)]
#[path = "resolved_tool_tests.rs"]
mod tests;
