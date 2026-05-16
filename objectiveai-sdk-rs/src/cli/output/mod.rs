//! Structured JSON Lines output for `objectiveai-cli`.
//!
//! Every line `objectiveai-cli` writes to stdout is one [`Output`] JSON
//! object. There are two top-level shapes, discriminated by `"type"`:
//!
//! - `error` — a failure or advisory ([`Error`]).
//! - `notification` — a typed payload `T` chosen by the consumer (the
//!   CLI defines its own notification enum and parameterizes
//!   `Output<T>` over it).
//!
//! `T` is flattened into the same JSON object as the `"type"` tag via
//! serde's internal tagging, so `T` should be a struct or an
//! internally-tagged enum.

mod error;
mod handle;
pub mod notification;

pub use error::*;
pub use handle::*;
pub use notification::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single line of CLI output.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.output.Output.{T}")]
pub enum Output<T> {
    Error(Error),
    /// Wraps `T` in [`Notification`] so its fields end up under a
    /// nested `value` key — protects against `T` carrying its own
    /// `"type"` field colliding with this enum's discriminator.
    Notification(Notification<T>),
    /// Stream-start marker. Wire: `{"type":"begin"}`.
    Begin,
    /// Stream-end marker. Wire: `{"type":"end"}`.
    End,
}

impl<T: Serialize> Output<T> {
    /// Emit this output via `handle`. Equivalent to
    /// `handle.emit(self).await`; see [`Handle::emit`] for the routing
    /// details.
    pub async fn emit(&self, handle: &Handle) {
        handle.emit(self).await
    }
}

#[cfg(test)]
mod tests;
