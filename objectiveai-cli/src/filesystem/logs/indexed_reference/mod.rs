//! `LogReference` for log files keyed by an `index` — used by
//! per-agent / per-invention completion wrappers that need to
//! preserve their position within a parent collection (a vector
//! completion's swarm-index, an invention's per-invention index,
//! etc.).
//!
//! Same `LogReference` name as [`super::LogReference`]; disambiguated
//! by module path. Access via:
//!
//! ```ignore
//! use crate::filesystem::logs::indexed_reference::LogReference;
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::LogReferenceTag;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.logs.indexed_reference.LogReference")]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
    pub index: u64,
}

impl LogReference {
    pub fn new(path: String, index: u64) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
            index,
        }
    }
}
