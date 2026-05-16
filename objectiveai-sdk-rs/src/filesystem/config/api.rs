use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ApiConfig")]
pub struct ApiConfig {
    #[serde(default)]
    pub mode: ApiMode,
    #[serde(skip_serializing_if = "ApiRemoteConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub remote: Option<ApiRemoteConfig>,
    #[serde(skip_serializing_if = "ApiLocalConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub local: Option<ApiLocalConfig>,
    #[serde(skip_serializing_if = "ApiHeadersConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub headers: Option<ApiHeadersConfig>,
}

impl ApiConfig {
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn remote(&mut self) -> &mut ApiRemoteConfig {
        self.remote.get_or_insert_with(ApiRemoteConfig::default)
    }

    pub fn local(&mut self) -> &mut ApiLocalConfig {
        self.local.get_or_insert_with(ApiLocalConfig::default)
    }

    pub fn headers(&mut self) -> &mut ApiHeadersConfig {
        self.headers.get_or_insert_with(ApiHeadersConfig::default)
    }

    pub fn get_mode(&self) -> ApiMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: ApiMode) {
        self.mode = mode;
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ApiMode")]
#[serde(rename_all = "snake_case")]
pub enum ApiMode {
    Remote,
    #[default]
    Local,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ApiRemoteConfig")]
pub struct ApiRemoteConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub objectiveai_address: Option<String>,
}

impl ApiRemoteConfig {
    pub fn is_empty(&self) -> bool {
        self.objectiveai_address.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_objectiveai_address(&self) -> Option<&str> {
        self.objectiveai_address.as_deref()
    }

    pub fn set_objectiveai_address(&mut self, value: impl Into<String>) {
        self.objectiveai_address = Some(value.into());
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ApiLocalConfig")]
pub struct ApiLocalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub claude_agent_sdk: Option<bool>,
}

impl ApiLocalConfig {
    pub fn is_empty(&self) -> bool {
        self.claude_agent_sdk.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_claude_agent_sdk(&self) -> Option<bool> {
        self.claude_agent_sdk
    }

    pub fn set_claude_agent_sdk(&mut self, value: bool) {
        self.claude_agent_sdk = Some(value);
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ApiHeadersConfig")]
pub struct ApiHeadersConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_objectiveai_authorization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_openrouter_authorization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_github_authorization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_mcp_authorization: Option<indexmap::IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_viewer_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_viewer_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub http_referer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_commit_author_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub x_commit_author_email: Option<String>,
}

impl ApiHeadersConfig {
    pub fn is_empty(&self) -> bool {
        self.x_objectiveai_authorization.is_none()
            && self.x_openrouter_authorization.is_none()
            && self.x_github_authorization.is_none()
            && self.x_mcp_authorization.as_ref().is_none_or(|m| m.is_empty())
            && self.x_viewer_signature.is_none()
            && self.x_viewer_address.is_none()
            && self.user_agent.is_none()
            && self.http_referer.is_none()
            && self.x_title.is_none()
            && self.x_commit_author_name.is_none()
            && self.x_commit_author_email.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_x_objectiveai_authorization(&self) -> Option<&str> { self.x_objectiveai_authorization.as_deref() }
    pub fn set_x_objectiveai_authorization(&mut self, value: impl Into<String>) { self.x_objectiveai_authorization = Some(value.into()); }

    pub fn get_x_openrouter_authorization(&self) -> Option<&str> { self.x_openrouter_authorization.as_deref() }
    pub fn set_x_openrouter_authorization(&mut self, value: impl Into<String>) { self.x_openrouter_authorization = Some(value.into()); }

    pub fn get_x_github_authorization(&self) -> Option<&str> { self.x_github_authorization.as_deref() }
    pub fn set_x_github_authorization(&mut self, value: impl Into<String>) { self.x_github_authorization = Some(value.into()); }

    pub fn get_x_mcp_authorization(&self) -> Option<&indexmap::IndexMap<String, String>> { self.x_mcp_authorization.as_ref() }
    pub fn add_x_mcp_authorization(&mut self, key: impl Into<String>, value: impl Into<String>) { self.x_mcp_authorization.get_or_insert_with(indexmap::IndexMap::new).insert(key.into(), value.into()); }
    pub fn del_x_mcp_authorization(&mut self, key: &str) { if let Some(mcp) = &mut self.x_mcp_authorization { mcp.shift_remove(key); } }

    pub fn get_x_viewer_signature(&self) -> Option<&str> { self.x_viewer_signature.as_deref() }
    pub fn set_x_viewer_signature(&mut self, value: impl Into<String>) { self.x_viewer_signature = Some(value.into()); }

    pub fn get_x_viewer_address(&self) -> Option<&str> { self.x_viewer_address.as_deref() }
    pub fn set_x_viewer_address(&mut self, value: impl Into<String>) { self.x_viewer_address = Some(value.into()); }

    pub fn get_user_agent(&self) -> Option<&str> { self.user_agent.as_deref() }
    pub fn set_user_agent(&mut self, value: impl Into<String>) { self.user_agent = Some(value.into()); }

    pub fn get_http_referer(&self) -> Option<&str> { self.http_referer.as_deref() }
    pub fn set_http_referer(&mut self, value: impl Into<String>) { self.http_referer = Some(value.into()); }

    pub fn get_x_title(&self) -> Option<&str> { self.x_title.as_deref() }
    pub fn set_x_title(&mut self, value: impl Into<String>) { self.x_title = Some(value.into()); }

    pub fn get_x_commit_author_name(&self) -> Option<&str> { self.x_commit_author_name.as_deref() }
    pub fn set_x_commit_author_name(&mut self, value: impl Into<String>) { self.x_commit_author_name = Some(value.into()); }

    pub fn get_x_commit_author_email(&self) -> Option<&str> { self.x_commit_author_email.as_deref() }
    pub fn set_x_commit_author_email(&mut self, value: impl Into<String>) { self.x_commit_author_email = Some(value.into()); }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
