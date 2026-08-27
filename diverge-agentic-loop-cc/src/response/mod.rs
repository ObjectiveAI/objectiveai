//! Everything Claude Code writes on its stream-json stdout.
//!
//! The harness runs `claude -p --output-format stream-json --verbose`
//! and reads NDJSON: one JSON record per line, guaranteed parseable —
//! Claude Code installs a stdout guard that diverts anything else to
//! stderr. These are those records, typed from the source's own
//! schemas, whole: the full `StdoutMessage` union, strict, so a line
//! that matches nothing is an error rather than a guess.

pub mod message;
