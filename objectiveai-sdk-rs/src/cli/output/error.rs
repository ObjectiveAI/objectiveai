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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.Error")]
pub struct Error {
    pub level: Level,
    pub fatal: bool,
    pub message: serde_json::Value,
    /// Stamped at emit time by [`super::Handle`] when its `agent_id`
    /// field is set; producers leave this `None` and let the handle
    /// fill it. Serde-skipped when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_id: Option<String>,
}

/// Severity matching the conventions used by bunyan / pino / `log` crate
/// JSON encoders. `fatal` is encoded separately on [`Error`].
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "cli.output.Level")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
