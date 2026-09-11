//! The fates not yet decided.

use std::sync::LazyLock;

use dashmap::DashMap;
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use tokio::sync::oneshot;

/// Every enqueued message's fate wire, by uuid, from the write that
/// queued it until the fate is decided — so the map's keys ARE the
/// queue as this container knows it.
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
pub static PENDING: LazyLock<
    DashMap<
        String,
        oneshot::Sender<Fate>,
    >,
> = LazyLock::new(DashMap::new);
