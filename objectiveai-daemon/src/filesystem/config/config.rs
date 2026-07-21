use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.Config")]
pub struct Config {
    #[serde(skip_serializing_if = "super::ApiConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub api: Option<super::ApiConfig>,
    #[serde(skip_serializing_if = "super::DaemonConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub daemon: Option<super::DaemonConfig>,
    #[serde(skip_serializing_if = "super::DbConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub db: Option<super::DbConfig>,
    #[serde(skip_serializing_if = "super::LaboratoriesConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratories: Option<super::LaboratoriesConfig>,
}

impl Config {
    pub fn api(&mut self) -> &mut super::ApiConfig {
        self.api.get_or_insert_with(super::ApiConfig::default)
    }

    pub fn daemon(&mut self) -> &mut super::DaemonConfig {
        self.daemon.get_or_insert_with(super::DaemonConfig::default)
    }

    pub fn db(&mut self) -> &mut super::DbConfig {
        self.db.get_or_insert_with(super::DbConfig::default)
    }

    pub fn laboratories(&mut self) -> &mut super::LaboratoriesConfig {
        self.laboratories
            .get_or_insert_with(super::LaboratoriesConfig::default)
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
