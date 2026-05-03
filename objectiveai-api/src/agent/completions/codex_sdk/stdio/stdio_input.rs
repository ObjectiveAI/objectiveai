use serde::Serialize;

use super::RunParams;

/// One line written to the runner's stdin. The caller picks an `id`
/// for each `Run`; the runner echoes that `id` on every outbound
/// `event`/`end`/`diag` for the request, allowing the caller to
/// demultiplex N concurrent streams.
///
/// Borrowed-everywhere shape — we never need to clone large inputs
/// (notably the user message body) just to ship a request.
///
/// In-flight cancellation is intentionally absent: codex's `Thread`
/// doesn't expose a stop point that doesn't leave a billing event
/// unaccounted for, so once a `Run` is sent the runner is allowed to
/// finish naturally.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StdioInput<'a> {
    /// Start a new in-flight stream.
    Run {
        id: &'a str,
        params: RunParams<'a>,
    },
}
