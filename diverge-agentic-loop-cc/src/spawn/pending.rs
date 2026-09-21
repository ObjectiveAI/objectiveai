//! The fates not yet decided.

use std::sync::LazyLock;

use dashmap::DashMap;
use diverge_container_proxy_sdk::agent::enqueue::Fate;
use rmcp::model::ContentBlock;
use tokio::sync::oneshot;

/// One enqueued message, from the write that queued it until its
/// fate is decided: the content as enqueued — what the `user` chunk
/// carries when the replay echo lands, since the echo carries Claude
/// Code's blocks and not MCP's — and the wire the fate goes out on.
pub struct Pending {
    /// The message's content, as enqueued.
    pub content: Vec<ContentBlock>,
    /// Where the fate goes.
    pub fate: oneshot::Sender<Fate>,
}

/// Every enqueued message, by uuid, from the write that queued it
/// until the fate is decided — so the map's keys ARE the queue as
/// this container knows it.
///
/// Beside the session lock, not inside it: resolving a fate is a
/// remove-and-send that never takes the lock. Entries are inserted
/// by an enqueue UNDER the lock — which is what makes a dequeue's
/// snapshot complete, since the dequeue holds the lock throughout —
/// and removed by whoever decides the fate: the dequeue whose cancel
/// reached it (dequeued) or found it already taken (delivered), the
/// reader on a replay echo (delivered) or at end of stream (missed),
/// or the enqueue itself tidying up after hearing back. A removal
/// that finds nothing, or a send nobody hears, is always skipped:
/// the fate was already decided, or the caller stopped listening.
pub static PENDING: LazyLock<DashMap<String, Pending>> = LazyLock::new(DashMap::new);
