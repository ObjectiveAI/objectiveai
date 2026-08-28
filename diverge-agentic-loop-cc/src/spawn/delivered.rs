//! The delivered-uuid set.

use std::sync::LazyLock;

use dashmap::DashSet;

/// Uuids of messages known to have entered the conversation.
///
/// Beside the session lock, not inside it: the reader will mark
/// replay echoes here (when the conversion work lands) without ever
/// taking the lock, which is what keeps it deadlock-free. Already
/// consulted: [`dequeue`](super::dequeue()) drops these from the
/// queued vector before writing cancels, so a queue whose every
/// message already landed is honestly empty.
pub static DELIVERED: LazyLock<DashSet<String>> =
    LazyLock::new(DashSet::new);
