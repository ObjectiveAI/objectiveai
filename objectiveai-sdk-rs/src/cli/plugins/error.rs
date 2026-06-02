//! Plugin-local copies of the structured error wire types. Same wire
//! shape as the cli's top-level error envelope, kept here so the plugin
//! protocol doesn't have to import from `cli::output`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A failure or advisory written to stdout by a plugin or echoed back
/// from the host. `fatal: true` means the emitter is exiting with a
/// non-zero status; `fatal: false` is a non-blocking warning.
///
/// `message` is an arbitrary JSON value so producers can emit structured
/// payloads. Wrap a plain string as `Value::String(...)` (or use
/// `.into()`) and the wire bytes stay identical to the old
/// `String`-only shape.
///
/// The `type` field is a single-variant [`ErrorType`] enum that always
/// serializes to `"error"`. This is what lets the untagged
/// [`super::CommandResponseValue`] enum reject every non-error wire
/// shape and fall through to its catch-all `Notification` variant.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.plugins.Error")]
pub struct Error {
    pub r#type: ErrorType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fatal: Option<bool>,
    pub message: serde_json::Value,
}

/// Single-variant discriminator for [`Error`]'s `type` field. Always
/// `"error"` on the wire.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.plugins.ErrorType")]
pub enum ErrorType {
    Error,
}

/// Severity matching the conventions used by bunyan / pino / `log` crate
/// JSON encoders. `fatal` is encoded separately on [`Error`].
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "cli.plugins.Level")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
