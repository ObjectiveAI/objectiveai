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
        let mut headers = indexmap::IndexMap::new();

        if !conn.session_id.is_empty() {
            headers.insert("Mcp-Session-Id".to_string(), conn.session_id.clone());
        }
        if let Some(auth) = &conn.authorization {
            headers.insert("Authorization".to_string(), auth.clone());
        }
        if !conn.user_agent.is_empty() {
            headers.insert("User-Agent".to_string(), conn.user_agent.clone());
        }
        if !conn.x_title.is_empty() {
            headers.insert("X-Title".to_string(), conn.x_title.clone());
        }
        if !conn.http_referer.is_empty() {
            headers.insert("Referer".to_string(), conn.http_referer.clone());
            headers.insert("HTTP-Referer".to_string(), conn.http_referer.clone());
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
