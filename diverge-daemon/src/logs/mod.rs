//! Every agent's log: what its run said, kept beside the records.
//!
//! An agent's log is a directory `<dir>/agents/<id>/` of two files:
//! `log`, one JSON object per line, each an
//! [`ItemWrapper`](diverge_sdk::daemon::endpoints::agents::logs::server::response::ItemWrapper)
//! as the wire sends it; and `index`, sixteen bytes per item — the
//! line's offset and its length, big-endian — so that a read from a
//! `logs_index` seeks rather than scans. Items are numbered from `1`
//! in the order appended, and the number of index entries is the
//! length of the log. [`append`] adds one item, which the caller
//! serializes under the agent's live [`Log`](crate::daemon::Log) lock;
//! [`count`] is the length; [`read`] is a span of items by index;
//! [`matches()`] is the request's filter over one item, the spans
//! aside; [`remove`] is the agent deleted. The log outlives the run
//! and is removed with the agent.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod append;
mod error;
mod filter;
mod read;
mod remove;

pub use append::*;
pub use error::*;
pub use filter::*;
pub use read::*;
pub use remove::*;

use std::path::{Path, PathBuf};

use crate::store::AgentId;

/// The agent's log directory under the daemon's `agents/`.
pub fn dir(root: &Path, id: AgentId) -> PathBuf {
    root.join(id.0.to_string())
}

/// The bytes one index entry takes: an offset and a length, each a
/// big-endian `u64`.
const ENTRY: u64 = 16;
