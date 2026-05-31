//! `Content` — polymorphic content shape. Mirrors
//! [`crate::agent::completions::message::RichContentLog`] /
//! [`crate::agent::completions::message::SimpleContentLog`]
//! flattened to bare [`Id`] integer refs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Id;

/// Either a single file (one text part or one media part) or a
/// list of files (multi-part content). Untagged: distinguishable on
/// the wire because [`Id`] is an integer and `Vec<Id>` is an array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "filesystem.logs.queue.Content")]
pub enum Content {
    #[schemars(title = "One")]
    One(Id),
    #[schemars(title = "Many")]
    Many(Vec<Id>),
}
