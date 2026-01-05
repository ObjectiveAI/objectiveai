use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Verbosity {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl Verbosity {
    pub fn prepare(self) -> Option<Self> {
        if let Verbosity::Medium = self {
            None
        } else {
            Some(self)
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}
