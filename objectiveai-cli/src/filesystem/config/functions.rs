use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.FunctionsConfig")]
pub struct FunctionsConfig {
    #[serde(skip_serializing_if = "FunctionsInventionsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub inventions: Option<FunctionsInventionsConfig>,
}

impl FunctionsConfig {
    pub fn is_empty(&self) -> bool {
        self.inventions.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn inventions(&mut self) -> &mut FunctionsInventionsConfig {
        self.inventions
            .get_or_insert_with(FunctionsInventionsConfig::default)
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.FunctionsInventionsConfig")]
pub struct FunctionsInventionsConfig {
    #[serde(default = "FunctionsInventionsConfig::default_remote")]
    pub remote: objectiveai_sdk::Remote,
}

impl Default for FunctionsInventionsConfig {
    fn default() -> Self {
        Self {
            remote: Self::default_remote(),
        }
    }
}

impl FunctionsInventionsConfig {
    fn default_remote() -> objectiveai_sdk::Remote {
        objectiveai_sdk::Remote::Filesystem
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.remote, objectiveai_sdk::Remote::Filesystem)
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_remote(&self) -> objectiveai_sdk::Remote {
        self.remote
    }

    pub fn set_remote(
        &mut self,
        remote: objectiveai_sdk::Remote,
    ) -> Result<(), super::super::Error> {
        if matches!(remote, objectiveai_sdk::Remote::Mock) {
            return Err(super::super::Error::InvalidRemote(remote));
        }
        self.remote = remote;
        Ok(())
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
