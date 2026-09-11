//! The channels the caller opens, each served on a task of its own.
//!
//! One loop reads the scope's inbox and hands every opened channel
//! to the task that serves it — after the id is out, and not before,
//! so a request the caller sent early waits in the inbox rather than
//! being answered against a container that is not up. The loop also
//! hears the three things that end a scope: the caller's stop, the
//! container leaving, and the caller going away.

pub(crate) mod agent;
pub(crate) mod filetree;
pub(crate) mod postgres;
pub(crate) mod read;
pub(crate) mod tool;
pub(crate) mod write;

mod serve;

pub(crate) use serve::*;
