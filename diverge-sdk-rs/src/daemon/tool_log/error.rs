//! An error the daemon met keeping a tool, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared;

/// An error of the daemon's own — what a tool's program made of its
/// rotating secrets that could not be kept, naming secrets and never a
/// value — kept under its own member so nothing in it is mistaken for
/// the item's `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Error {
    /// The error, whole.
    pub error: shared::error::Error,
}
