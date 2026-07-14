use serde::{Deserialize, Serialize};

/// The `laboratories` config section: how `laboratories spawn` wires
/// the machine's resident laboratory HOST to daemons.
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.LaboratoriesConfig")]
pub struct LaboratoriesConfig {
    /// Daemon `http://` address → signature to present there. An EMPTY
    /// signature dials unauthenticated (that daemon must be open).
    /// Every entry gets its own resident connection from the host (which
    /// re-derives `ws://` to open the `/laboratory` WebSocket).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub addresses: Option<indexmap::IndexMap<String, String>>,
    /// Whether the host connects to the LOCAL daemon on spawn.
    /// Unset ⇒ true. `false` ⇒ spawn dials only the configured
    /// `addresses`, and local auto-spawns (create/delete targeting
    /// this machine) error instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub local: Option<bool>,
}

impl LaboratoriesConfig {
    pub fn is_empty(&self) -> bool {
        self.addresses.as_ref().is_none_or(|m| m.is_empty()) && self.local.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_addresses(&self) -> Option<&indexmap::IndexMap<String, String>> {
        self.addresses.as_ref()
    }
    pub fn add_address(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.addresses
            .get_or_insert_with(indexmap::IndexMap::new)
            .insert(key.into(), value.into());
    }
    pub fn del_address(&mut self, key: &str) {
        if let Some(addresses) = &mut self.addresses {
            addresses.shift_remove(key);
        }
    }

    pub fn get_local(&self) -> Option<bool> {
        self.local
    }
    pub fn set_local(&mut self, value: bool) {
        self.local = Some(value);
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
