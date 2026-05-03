use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpStdioServerConfigType {
    Stdio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpStdioServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<McpStdioServerConfigType>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<indexmap::IndexMap<String, String>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpSSEServerConfigType {
    Sse,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpSSEServerConfig {
    pub r#type: McpSSEServerConfigType,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<indexmap::IndexMap<String, String>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpHttpServerConfigType {
    Http,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpHttpServerConfig {
    pub r#type: McpHttpServerConfigType,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<indexmap::IndexMap<String, String>>,
}

impl From<&objectiveai::mcp::Connection> for McpHttpServerConfig {
    fn from(conn: &objectiveai::mcp::Connection) -> Self {
        // The connection's `headers` field is the same merged map the
        // proxy stamps on every request — User-Agent / X-Title /
        // Referer / HTTP-Referer / Authorization / any custom X-*.
        // Add `Mcp-Session-Id` on top so the SDK reuses the same
        // session.
        let mut headers = conn.headers.clone();
        if !conn.session_id.is_empty() {
            headers.insert("Mcp-Session-Id".to_string(), conn.session_id.clone());
        }

        McpHttpServerConfig {
            r#type: McpHttpServerConfigType::Http,
            url: conn.url.clone(),
            headers: if headers.is_empty() {
                None
            } else {
                Some(headers)
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    SSE(McpSSEServerConfig),
    Http(McpHttpServerConfig),
}
