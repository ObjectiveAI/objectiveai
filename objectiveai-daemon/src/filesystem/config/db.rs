use serde::{Deserialize, Serialize};

/// Defaults applied wherever a `DbConfig` field is unset. They match
/// objectiveai-db's own env defaults, so a fresh install can
/// the daemon's on-demand objectiveai-db and connect with zero config.
pub const DB_DEFAULT_ADDRESS: &str = "127.0.0.1";
pub const DB_DEFAULT_USER: &str = "postgres";
pub const DB_DEFAULT_PASSWORD: &str = "objectiveai";
pub const DB_DEFAULT_DATABASE: &str = "objectiveai";

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.DbConfig")]
pub struct DbConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub database: Option<String>,
}

impl DbConfig {
    pub fn is_empty(&self) -> bool {
        self.address.is_none()
            && self.user.is_none()
            && self.password.is_none()
            && self.database.is_none()
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


    pub fn get_user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub fn set_user(&mut self, value: impl Into<String>) {
        self.user = Some(value.into());
    }

    pub fn get_password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn set_password(&mut self, value: impl Into<String>) {
        self.password = Some(value.into());
    }

    pub fn get_database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    pub fn set_database(&mut self, value: impl Into<String>) {
        self.database = Some(value.into());
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
