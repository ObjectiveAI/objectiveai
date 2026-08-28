//! The fates not yet decided.

use std::sync::LazyLock;

use dashmap::DashMap;
use diverge_provider_sdk::agentic_loop_container;
use tokio::sync::{Mutex, oneshot};

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
/// reader at end of stream (missed), the main loop on a replay echo
/// (delivered — when the conversion work lands), or the enqueue
/// itself tidying up after hearing back. A removal that finds
/// nothing, or a send nobody hears, is always skipped: the fate was
/// already decided, or the caller stopped listening.
pub static PENDING: LazyLock<
    DashMap<
        String,
        oneshot::Sender<agentic_loop_container::enqueue::Response>,
    >,
> = LazyLock::new(DashMap::new);

/// The withdrawal boundary: no dequeue may cancel a message enqueued
/// after it arrived.
///
/// Both verbs take this FIRST — a dequeue the moment it starts (that
/// acquisition is "the cancel came in"), holding it until it has
/// read [`PENDING`]'s uuids; an enqueue before doing anything,
/// holding it until its fate is registered. Tokio's mutex is a FIFO
/// queue, so arrival order IS the ordering: an enqueue ahead of a
/// dequeue in the queue registers before the snapshot and is
/// withdrawn; one behind it waits the snapshot out and is never
/// cancelled. Everything else (the initial prompt, the reader) stays
/// off it.
///
/// Lock order: whoever holds this and a session lock acquired the
/// gate FIRST — both verbs do — which is what keeps the module free
/// of ordering cycles.
pub static GATE: Mutex<()> = Mutex::const_new(());
