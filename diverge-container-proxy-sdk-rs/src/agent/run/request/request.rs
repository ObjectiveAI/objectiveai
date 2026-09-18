//! What one loop is asked.

use serde::{Deserialize, Serialize};

/// Run one loop on this prompt.
///
/// The arguments are not here: they were registered once, and they
/// never change. The prompt is each loop's, because a container runs
/// loops one after another — each resuming the conversation the last
/// one left — and every one is asked something. When several
/// messages waited for the loop, the prompt is all of them, joined
/// with a blank line between, in the order they were enqueued.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// What the loop is asked.
    pub prompt: String,
}
