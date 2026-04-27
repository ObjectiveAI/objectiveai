//! Wire types for the `objectiveai-claude-agent-sdk-runner-py` stdio
//! NDJSON protocol.
//!
//! The runner is a long-lived stdio server that handles many concurrent
//! requests over a single (stdin, stdout, stderr) triple. The caller
//! tags every `run` request with a string `id`; every line emitted on
//! stdout and stderr carries the same `id` so the caller can
//! demultiplex N concurrent streams.
//!
//! ## Wire shapes
//!
//! Every line is one JSON object terminated by `\n`. By convention,
//! **`type` is the first field, `id` is the second field** on every
//! tagged line. This lets readers locate the id with a small byte
//! scan past the type discriminator and reject foreign lines without
//! fully deserializing the rest. The single exception is
//! process-level `fatal` lines (no `id`), which are emitted on stderr
//! only when the runner is exiting non-zero — those have `type` first
//! and `message` (or any other field) second.
//!
//! Stdout (per-request, both variants of [`StdioOutput`] carry `id`):
//!
//! ```text
//! {"type":"event","id":"<id>","event":<T>}
//! {"type":"end","id":"<id>","status":"ok"}
//! {"type":"end","id":"<id>","status":"cancelled"}
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
//! ## Routing
//!
//! The [`Runner`] dispatcher full-deserializes each stdout line into a
//! [`StdioOutput<SDKMessage>`][StdioOutput] (and each stderr line into
//! a [`StdioError`]) and routes by the `id` field on the parsed value.
//! There's no separate scan layer — every line on these pipes belongs
//! to one of our active requests, so there's nothing to early-exit on.

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
