//! `Id` — a compact SQL row id pointing into the `files` table.
//! Lazily populated by
//! [`crate::filesystem::Client::read_new_from_queue`]: the first
//! time a given on-disk path is referenced from a queue read it
//! gets a row in `files` (`INSERT … ON CONFLICT(path) DO UPDATE
//! … RETURNING id`); every later read returns the same id. Use
//! [`crate::filesystem::Client::path_for_file_id`] to resolve an
//! `Id` back to its logs-relative path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable integer id of a logged file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(rename = "filesystem.logs.queue.Id")]
pub struct Id(pub i64);
