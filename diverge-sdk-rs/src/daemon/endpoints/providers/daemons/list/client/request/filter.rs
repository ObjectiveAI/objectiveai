//! What narrows a list of daemons.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// The filter over remote daemons: every member optional, and every
/// one given a condition a daemon passes or does not. A filter with no
/// member given passes every daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Any one of these names. Absent when empty, and then any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// Linked through any one of these providers, by identity. Absent
    /// when empty, and then any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<Identity>,
    /// Whether this daemon holds a connection to it now: `true`, or
    /// none, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
}
