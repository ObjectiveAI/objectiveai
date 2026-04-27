use serde::Serialize;

use super::RunParams;

/// One line written to the runner's stdin. The caller picks an `id`
/// for each `Run`; the runner echoes that `id` on every outbound
/// `event`/`end`/`diag` for the request, allowing the caller to
/// demultiplex N concurrent streams.
///
/// Borrowed-everywhere shape — we never need to clone large inputs
/// (notably the SDK user message body) just to ship a request.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StdioInput<'a> {
    /// Start a new in-flight stream.
    Run {
        id: &'a str,
        params: RunParams<'a>,
    },
    /// Best-effort abort of one in-flight stream.
    Cancel { id: &'a str },
}
