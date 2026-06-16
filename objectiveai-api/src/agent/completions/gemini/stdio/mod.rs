//! Wire types for the `objectiveai-gemini-sdk-runner` stdio NDJSON
//! protocol. The OUTER envelope is identical to the codex / claude
//! runners — one long-lived Python subprocess multiplexes N concurrent
//! `run` calls over a single (stdin, stdout, stderr) triple. The caller
//! tags every `run` request with a string `id`; every line emitted on
//! stdout and stderr carries the same `id` so the caller can
//! demultiplex N concurrent streams. Only the inner `event` payload
//! type differs ([`super::GeminiEvent`] instead of codex's
//! `ThreadEvent`).
//!
//! ## Wire shapes
//!
//! Every line is one JSON object terminated by `\n`. By convention,
//! **`type` is the first field, `id` is the second field** on every
//! tagged line.
//!
//! Stdout (per-request, both variants of [`StdioOutput`] carry `id`):
//!
//! ```text
//! {"type":"event","id":"<id>","event":<GeminiEvent>}
//! {"type":"end","id":"<id>","status":"ok"}
//! {"type":"end","id":"<id>","status":"error","error":"<msg>"}
//! ```
//!
//! Stderr (per-request `diag` carries `id`; process-fatal does not):
//!
//! ```text
//! {"type":"diag","id":"<id>","level":"info|warn|error","message":"..."}
//! {"type":"fatal","message":"..."}
//! ```
//!
//! The gemini runner is STATELESS — it receives the full conversation
//! on every `run` request and holds no session. In-flight cancellation
//! is intentionally absent: once a `run` is sent the runner is allowed
//! to finish naturally.

mod run_params;
pub use run_params::*;
mod runner;
pub use runner::*;
mod runner_error;
pub use runner_error::*;
mod runner_stream;
pub use runner_stream::*;
mod runner_update;
pub use runner_update::*;
mod stdio_diag_level;
pub use stdio_diag_level::*;
mod stdio_end_status;
pub use stdio_end_status::*;
mod stdio_error;
pub use stdio_error::*;
mod stdio_input;
pub use stdio_input::*;
mod stdio_output;
pub use stdio_output::*;
