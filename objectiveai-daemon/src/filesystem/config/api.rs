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
    /// MCP CONNECT timeout override, in milliseconds, projected onto the
    /// spawned API's `MCP_CONNECT_TIMEOUT` env. That is the only MCP
    /// timeout left to configure: the daemon's own MCP calls and the
    /// API's per-call MCP waits are unbounded (wait forever), and the
    /// proxy keeps its own crate-internal call-timeout default. `None`
    /// ⇒ the API resolves its own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_timeout_ms: Option<u64>,
    /// Backoff max-elapsed-time override, in milliseconds. One value,
    /// fanned out to EVERY backoff retry policy of the spawned API
    /// (`AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME`,
    /// `MCP_BACKOFF_MAX_ELAPSED_TIME`, `GITHUB_BACKOFF_MAX_ELAPSED_TIME`)
    /// and used for the CLI's own MCP client backoff. `None` ⇒ the
    /// canonical default (60000ms). The other exponential-backoff knobs
    /// (intervals / randomization / multiplier) keep their built-in
    /// defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub backoff_max_elapsed_time_ms: Option<u64>,
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
            && self.mcp_timeout_ms.is_none()
            && self.backoff_max_elapsed_time_ms.is_none()
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

    pub fn get_mcp_timeout_ms(&self) -> Option<u64> {
        self.mcp_timeout_ms
    }
    pub fn set_mcp_timeout_ms(&mut self, value: u64) {
        self.mcp_timeout_ms = Some(value);
    }

    pub fn get_backoff_max_elapsed_time_ms(&self) -> Option<u64> {
        self.backoff_max_elapsed_time_ms
    }
    pub fn set_backoff_max_elapsed_time_ms(&mut self, value: u64) {
        self.backoff_max_elapsed_time_ms = Some(value);
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
