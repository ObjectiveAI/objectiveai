use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.McpConfig")]
pub struct McpConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub port: Option<u16>,
}

impl McpConfig {
    pub fn is_empty(&self) -> bool {
        self.address.is_none() && self.port.is_none()
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

    pub fn get_port(&self) -> Option<u16> {
        self.port
    }

    pub fn set_port(&mut self, value: u16) {
        self.port = Some(value);
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
