//! The channels the caller opens, each served against the proxy on a
//! task of its own.
//!
//! One loop reads the scope's inbox and hands every opened channel
//! to the task that serves it — after the id is out, and not before,
//! so a request the caller sent early waits in the inbox rather than
//! being answered against a container that is not up. The loop also
//! hears the three things that end a scope: the caller's stop, the
//! container leaving, and the caller going away. A tree, a read and a
//! write are scopes this end opens on the proxy, and a transfer is a
//! read on this proxy wired into a write on another container's; a
//! database half, the image's `/DIVERGE.md`, a feed of the secrets
//! and the family's own exchange are channels on the begin scope; a FUSE
//! mount asked for while the container runs is one more mount scope
//! on the proxy, waited for.

pub(crate) mod agent;
pub(crate) mod daemon;
pub(crate) mod filetree;
pub(crate) mod fuse_mount;
pub(crate) mod info;
pub(crate) mod postgres;
pub(crate) mod read;
pub(crate) mod secrets;
pub(crate) mod tool;
pub(crate) mod transfer;
pub(crate) mod write;

mod serve;

pub(crate) use serve::*;
