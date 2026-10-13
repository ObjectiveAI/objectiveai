//! The `run` command: one request to a daemon, its answers printed.
//!
//! The selected daemon is [`dial`]ed as the selected account; the
//! [`Request`] is made of the name and the body; and [`endpoints::run`]
//! puts it — one of the ninety-eight, through the [`shape`] its
//! exchange has — printing every answer frame through the
//! [`Emitter`] as one compact JSON line, each through the jq filter
//! when there is one. Nothing stays open past its answer: a list ends
//! at its `listed` frame and a filetree at its first snapshot; a logs
//! request runs as far as it asked — to the log's end, or, watching,
//! as many items as its `count` — and a connect puts the one MCP
//! exchange `--mcp` names and disconnects. Bytes never cross stdout: an upload
//! reads under, and a download writes under, the local path `--path`
//! names, which those requests require and every other refuses. A
//! timeout abandons the request and ends the run with [`Error::Timeout`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod dial;
mod emit;
mod error;
mod refusing;
mod request;
mod run;

pub mod endpoints;
pub mod shape;

pub use dial::*;
pub use emit::*;
pub use error::*;
pub use refusing::*;
pub use request::*;
pub use run::*;
