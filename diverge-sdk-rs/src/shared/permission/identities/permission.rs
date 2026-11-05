//! A grant over the host's identities.

use serde::{Deserialize, Serialize};

use super::Action;

/// The acts a grant allows over every identity of the host's: as JSON,
/// the list of them, `["issuers", "subjects", "login"]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
