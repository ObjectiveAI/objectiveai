//! The `thread.started` event: the thread, named.

use serde::Deserialize;

/// A `type: "thread.started"` event — the first event of every
/// process, fresh or resumed, written when the session is
/// configured.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ThreadStarted {
    /// Always `thread.started`.
    pub r#type: ThreadStartedType,
    /// The thread's id: what `codex exec resume` takes to pick the
    /// conversation up, and so the continuation's key.
    pub thread_id: String,
}

/// The `thread.started` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum ThreadStartedType {
    /// The only value.
    #[default]
    #[serde(rename = "thread.started")]
    ThreadStarted,
}
