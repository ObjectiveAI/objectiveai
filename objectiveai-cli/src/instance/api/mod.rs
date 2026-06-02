mod args;
pub use args::*;
mod body;
pub use body::*;
pub mod conduit;

/// Process exit code the instance runner uses when the per-agent socket
/// is already owned by a live listener (admission-gate loss). The cli's
/// `api/stream_subprocess.rs` maps this code to `Error::CliStreamSlotTaken`
/// and the dispatch entry retries.
pub const SLOT_TAKEN_EXIT_CODE: i32 = 42;
