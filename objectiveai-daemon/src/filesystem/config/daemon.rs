use serde::{Deserialize, Serialize};

/// The `daemon` config section: the resident daemon's client-facing
/// coordinates. NOT consumed by anything yet — the daemon still binds
/// from its bare `ADDRESS`/`PORT`/`SECRET`/`SIGNATURE` env; these
/// fields exist so `daemon config …` commands (and the
/// `refresh-secret-signature-pair` rotation) have a durable home ahead
/// of that wiring.
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.DaemonConfig")]
pub struct DaemonConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub signature: Option<String>,
}

impl DaemonConfig {
    pub fn is_empty(&self) -> bool {
        self.address.is_none() && self.secret.is_none() && self.signature.is_none()
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

    pub fn get_secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    pub fn set_secret(&mut self, value: impl Into<String>) {
        self.secret = Some(value.into());
    }

    pub fn get_signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    pub fn set_signature(&mut self, value: impl Into<String>) {
        self.signature = Some(value.into());
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
