//! One line of stdout, whichever event it is.

use serde::Deserialize;

use super::{
    Error, ItemCompleted, ItemStarted, ItemUpdated, ThreadStarted, TurnCompleted, TurnFailed,
    TurnStarted,
};

/// Everything `codex exec --json` can write, one line at a time —
/// the source's `ThreadEvent`, whole.
///
/// Untagged, the way this crate models unions: every event carries
/// its own `type` literal as a marker field, so an event is
/// self-describing wherever it travels and only the right variant
/// can accept a given literal. What the source's tagged enum hoists
/// out of the leaf stays on the leaf here. The eight variants are
/// the pinned version's complete vocabulary; there is no catch-all,
/// as the module states.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ThreadEvent {
    /// The thread, named — the first event of every process.
    ThreadStarted(ThreadStarted),
    /// A turn beginning.
    TurnStarted(TurnStarted),
    /// A turn ending well, with the thread's usage.
    TurnCompleted(TurnCompleted),
    /// A turn ending in failure.
    TurnFailed(TurnFailed),
    /// An item beginning, in progress.
    ItemStarted(ItemStarted),
    /// An item changing — only the todo list does.
    ItemUpdated(ItemUpdated),
    /// An item reaching its terminal state.
    ItemCompleted(ItemCompleted),
    /// A critical error, not by itself the turn's end.
    Error(Error),
}
