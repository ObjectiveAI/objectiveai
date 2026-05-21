use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.Config")]
pub struct Config {
    #[serde(skip_serializing_if = "super::ApiConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub api: Option<super::ApiConfig>,
    #[serde(skip_serializing_if = "super::AgentsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agents: Option<super::AgentsConfig>,
    #[serde(skip_serializing_if = "super::SwarmsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub swarms: Option<super::SwarmsConfig>,
    #[serde(skip_serializing_if = "super::FunctionsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub functions: Option<super::FunctionsConfig>,
    #[serde(skip_serializing_if = "super::ViewerConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer: Option<super::ViewerConfig>,
    #[serde(skip_serializing_if = "super::McpConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp: Option<super::McpConfig>,
}

impl Config {
    pub fn api(&mut self) -> &mut super::ApiConfig {
        self.api.get_or_insert_with(super::ApiConfig::default)
    }

    pub fn agents(&mut self) -> &mut super::AgentsConfig {
        self.agents.get_or_insert_with(super::AgentsConfig::default)
    }

    pub fn swarms(&mut self) -> &mut super::SwarmsConfig {
        self.swarms.get_or_insert_with(super::SwarmsConfig::default)
    }

    pub fn functions(&mut self) -> &mut super::FunctionsConfig {
        self.functions.get_or_insert_with(super::FunctionsConfig::default)
    }

    pub fn viewer(&mut self) -> &mut super::ViewerConfig {
        self.viewer.get_or_insert_with(super::ViewerConfig::default)
    }

    pub fn mcp(&mut self) -> &mut super::McpConfig {
        self.mcp.get_or_insert_with(super::McpConfig::default)
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
