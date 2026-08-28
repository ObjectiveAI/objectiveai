//! The Claude Code subprocess: spawning it, and the queue verbs
//! against it.

mod spawn;
mod stdin;

// `spawn` itself has no caller until the root handler lands; the
// allow leaves with it.
#[allow(unused_imports)]
pub use spawn::{dequeue, enqueue, spawn};
