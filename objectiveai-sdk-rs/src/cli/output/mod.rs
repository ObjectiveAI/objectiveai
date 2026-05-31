//! Structured JSON Lines output for `objectiveai-cli`.
//!
//! Every line `objectiveai-cli` writes to stdout is one [`Output`] JSON
//! object. There are two top-level shapes, discriminated by `"type"`:
//!
//! - `error` — a failure or advisory ([`Error`]).
//! - `notification` — a typed payload ([`Notification`] wrapping
//!   [`NotificationValue`]). The inner enum's `kind` tag discriminates
//!   the concrete variant so a consumer can do a single
//!   `serde_json::from_str::<Output>(line)` and dispatch.

mod error;
mod handle;
pub mod notification;

pub use error::*;
pub use handle::*;
pub use notification::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single line of CLI output.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.output.Output")]
pub enum Output {
    Error(Error),
    /// Wraps [`NotificationValue`] in [`Notification`] so its fields
    /// end up under a nested `value` key.
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
