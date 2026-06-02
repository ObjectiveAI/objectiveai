use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A failure or advisory written to stdout. `fatal: true` means the
/// process is exiting with a non-zero status; `fatal: false` is a
/// non-blocking warning (e.g. auto-update failed but the requested
/// command still ran).
///
/// `message` is an arbitrary JSON value so producers can emit
/// structured payloads (e.g. `{"code": ..., "detail": ...}`). Wrap
/// a plain string as `Value::String(...)` (or use `.into()`) and the
/// wire bytes stay identical to the old `String`-only shape.
///
/// The `type` field is a single-variant `ErrorType` enum that
/// always serializes to `"error"`. This is what disambiguates the
/// untagged `Output` enum from a notification — `Output` tries
/// `Error` first, and the constant `type:"error"` tag is what
/// rejects every non-error wire shape.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.Error")]
pub struct Error {
    pub r#type: ErrorType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fatal: Option<bool>,
    pub message: serde_json::Value,
}

/// Single-variant discriminator for [`Error`]'s `type` field.
/// Always `"error"` on the wire.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.output.ErrorType")]
pub enum ErrorType {
    Error,
}

/// Severity matching the conventions used by bunyan / pino / `log` crate
/// JSON encoders. `fatal` is encoded separately on [`Error`].
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "cli.output.Level")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
