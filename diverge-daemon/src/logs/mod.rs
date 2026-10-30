//! Every log the daemon keeps: each agent's, what its run said, and
//! each tool's, what the daemon routed into it — kept beside the
//! records.
//!
//! Whose log it is, is a [`Book`]: an agent by its row, its log a
//! directory `<dir>/agents/<id>/`; a tool on record by its row,
//! `<dir>/tools/<id>/`; a dependency tool by its row,
//! `<dir>/dependency_tools/<id>/` — three directories, since the rows
//! are numbered apart and would meet in one. Each log is two files:
//! `log`, one JSON object per line, each an [`Entry`] as the wire
//! sends it; and `index`, sixteen bytes per item — the line's offset
//! and its length, big-endian — so that a read from a `logs_index`
//! seeks rather than scans. Items are numbered from `1` in the order
//! appended, and the number of index entries is the length of the
//! log. [`append`] adds one item, which the caller serializes under
//! the log's live lock — [`Log`](crate::daemon::Log) for an agent's,
//! [`ToolLog`](crate::daemon::ToolLog) for a tool's; [`count`] is the
//! length; [`read`] is a span of items by index; [`matches()`] and
//! [`tool_matches`] are a request's filter over one item, the spans
//! aside; [`remove`] is the owner deleted. A log outlives every run
//! and is removed with its owner.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod append;
mod book;
mod error;
mod filter;
mod read;
mod remove;

pub use append::*;
pub use book::*;
pub use error::*;
pub use filter::*;
pub use read::*;
pub use remove::*;

/// The bytes one index entry takes: an offset and a length, each a
/// big-endian `u64`.
const ENTRY: u64 = 16;
