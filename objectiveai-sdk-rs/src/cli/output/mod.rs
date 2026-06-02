//! Structured JSON Lines output for `objectiveai-cli`.
//!
//! Every line `objectiveai-cli` writes to stdout is one [`Output`] JSON
//! object. The enum is `#[serde(untagged)]` — there is no
//! `type:"notification"` envelope. Deserialization tries [`Error`]
//! first (its `type` field is a single-variant `ErrorType` forcing
//! `"error"`, so non-error wire shapes reject fast), then
//! [`Notification`] (which flattens [`NotificationValue`]'s `type`
//! tag and the catch-all `Other` map directly at the top level).

mod handle;
pub mod notification;

pub use handle::*;
pub use notification::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single line of CLI output. Untagged — each variant carries its
/// own internal discriminator on the wire (Error has `type:"error"`,
/// Notification's flattened `NotificationValue` either has
/// `type:"<typed-variant>"` or is a raw `Other` map).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.output.Output")]
pub enum Output {
    /// Try Error first: its `type` field is a single-variant
    /// `ErrorType` (always `"error"`), so any non-error wire shape
    /// fails deserialization quickly and falls through to
    /// `Notification`. Putting `Notification` first would mean the
    /// untagged `NotificationValue::Other` catch-all silently swallows
    /// Error payloads.
    #[schemars(title = "Error")]
    Error(Error),
    #[schemars(title = "Notification")]
    Notification(Notification),
}

impl Output {
    /// Emit this output via `handle`. Equivalent to
    /// `handle.emit(self).await`; see [`Handle::emit`] for the routing
    /// details.
    pub async fn emit(&self, handle: &Handle) {
        handle.emit(self).await
    }
}

#[cfg(test)]
mod tests;
