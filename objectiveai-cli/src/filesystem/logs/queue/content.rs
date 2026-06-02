//! `Content` — polymorphic content shape. Mirrors
//! [`objectiveai_sdk::agent::completions::message::RichContentLog`] /
//! [`objectiveai_sdk::agent::completions::message::SimpleContentLog`]
//! flattened to bare integer file-id refs (SQL row ids into the
//! `files` table — see [`crate::filesystem::Client::read_new_from_queue`]).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Either a single file (one text part or one media part) or a
/// list of files (multi-part content). Untagged: distinguishable on
/// the wire because a single id is an integer and `Vec<i64>` is an array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "filesystem.logs.queue.Content")]
pub enum Content {
    #[schemars(title = "One")]
    One(i64),
    #[schemars(title = "Many")]
    Many(Vec<i64>),
}
