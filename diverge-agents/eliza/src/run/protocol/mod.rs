//! The line protocol between this program and the entry process:
//! one JSON object per line, each way.
//!
//! [`Request`] is what this program writes to the entry's stdin —
//! `configure` once, then `turn`, `read` and `stop`; [`Response`] is
//! what the entry writes to its stdout — `ready` once, the turn's
//! stream, `done`, `value`, `stopped`, or `fatal` before ready. Both
//! are discriminated by `type`. The entry is `node/entry.mjs`, and
//! this module is its Rust half; the two must agree, and there is no
//! third copy.

mod read_kind;
mod request;
mod response;

pub use read_kind::*;
pub use request::*;
pub use response::*;
