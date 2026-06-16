use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.ApiConfig")]
pub struct ApiConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub objectiveai_authorization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub openrouter_authorization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub github_authorization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_authorization: Option<indexmap::IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub user_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub http_referer: Option<String>,
    /// `X-Title` HTTP header — keeps the `x_` prefix because that's the
    /// literal header name (every other field had its `x_` dropped on
    /// the flatten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub commit_author_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub commit_author_email: Option<String>,
    /// MCP retry-backoff override (a single JSON blob). Drives BOTH the
    /// CLI's own MCP client (the streaming conduit, through which every
    /// CLI MCP connection flows) AND the `MCP_BACKOFF_*` env the CLI
    /// projects onto the spawned API (which in turn drives the proxy it
    /// spawns). `None` ⇒ the canonical default
    /// ([`objectiveai_sdk::mcp::Backoff::default`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_backoff: Option<objectiveai_sdk::mcp::Backoff>,
}

impl ApiConfig {
    pub fn is_empty(&self) -> bool {
        self.address.is_none()
            && self.objectiveai_authorization.is_none()
            && self.openrouter_authorization.is_none()
            && self.github_authorization.is_none()
            && self.mcp_authorization.as_ref().is_none_or(|m| m.is_empty())
            && self.user_agent.is_none()
            && self.http_referer.is_none()
            && self.x_title.is_none()
            && self.commit_author_name.is_none()
            && self.commit_author_email.is_none()
            && self.mcp_backoff.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_address(&self) -> Option<&str> {
        self.address.as_deref()
    }
    pub fn set_address(&mut self, value: impl Into<String>) {
        self.address = Some(value.into());
    }




    pub fn get_objectiveai_authorization(&self) -> Option<&str> {
        self.objectiveai_authorization.as_deref()
    }
    pub fn set_objectiveai_authorization(&mut self, value: impl Into<String>) {
        self.objectiveai_authorization = Some(value.into());
    }

    pub fn get_openrouter_authorization(&self) -> Option<&str> {
        self.openrouter_authorization.as_deref()
    }
    pub fn set_openrouter_authorization(&mut self, value: impl Into<String>) {
        self.openrouter_authorization = Some(value.into());
    }

    pub fn get_github_authorization(&self) -> Option<&str> {
        self.github_authorization.as_deref()
    }
    pub fn set_github_authorization(&mut self, value: impl Into<String>) {
        self.github_authorization = Some(value.into());
    }

    pub fn get_mcp_authorization(
        &self,
    ) -> Option<&indexmap::IndexMap<String, String>> {
        self.mcp_authorization.as_ref()
    }
    pub fn add_mcp_authorization(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.mcp_authorization
            .get_or_insert_with(indexmap::IndexMap::new)
            .insert(key.into(), value.into());
    }
    pub fn del_mcp_authorization(&mut self, key: &str) {
        if let Some(mcp) = &mut self.mcp_authorization {
            mcp.shift_remove(key);
        }
    }

    pub fn get_user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
    pub fn set_user_agent(&mut self, value: impl Into<String>) {
        self.user_agent = Some(value.into());
    }

    pub fn get_http_referer(&self) -> Option<&str> {
        self.http_referer.as_deref()
    }
    pub fn set_http_referer(&mut self, value: impl Into<String>) {
        self.http_referer = Some(value.into());
    }

    pub fn get_x_title(&self) -> Option<&str> {
        self.x_title.as_deref()
    }
    pub fn set_x_title(&mut self, value: impl Into<String>) {
        self.x_title = Some(value.into());
    }

    pub fn get_commit_author_name(&self) -> Option<&str> {
        self.commit_author_name.as_deref()
    }
    pub fn set_commit_author_name(&mut self, value: impl Into<String>) {
        self.commit_author_name = Some(value.into());
    }

    pub fn get_commit_author_email(&self) -> Option<&str> {
        self.commit_author_email.as_deref()
    }
    pub fn set_commit_author_email(&mut self, value: impl Into<String>) {
        self.commit_author_email = Some(value.into());
    }

    pub fn get_mcp_backoff(&self) -> Option<objectiveai_sdk::mcp::Backoff> {
        self.mcp_backoff
    }
    pub fn set_mcp_backoff(&mut self, value: objectiveai_sdk::mcp::Backoff) {
        self.mcp_backoff = Some(value);
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
