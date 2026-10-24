//! What may be done to providers.

use serde::{Deserialize, Serialize};

/// An act over the daemon's providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Read one, by identity.
    Get,
    /// List them.
    List,
}
