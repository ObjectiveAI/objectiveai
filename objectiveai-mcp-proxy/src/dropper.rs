//! Caller-driven forceful teardown + permanent ban of a response id.
//!
//! The embedder (the API) constructs a [`Dropper`], passes a clone into
//! [`crate::setup`], and keeps a clone to invoke. Calling
//! [`Dropper::drop`] for a `response_id`:
//!   1. **forever-bans** it (a concurrency-safe set checked on every
//!      `initialize`), then
//!   2. tears down any live session for it.
//!
//! Banning before teardown — paired with `handle_initialize` inserting
//! before it checks the ban — guarantees no `initialize` racing a `drop`
//! can leave a session alive (see the module's teardown / the
//! `handle_initialize` checks).
//!
//! The teardown context (the [`SessionManager`] and the optional
//! [`ReverseChannel`]) only exists once `setup` has built it, after the
//! caller already holds the `Dropper`. It is therefore late-bound via a
//! `OnceLock`, wired exactly once by `setup` ([`Dropper::wire`]).

use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use crate::ReverseChannel;
use crate::session_manager::SessionManager;

/// Cheaply-cloneable handle for banning + dropping response ids. Shared
/// between the caller (who invokes [`Self::drop`]) and the proxy (which
/// checks [`Self::is_banned`] on every `initialize`).
#[derive(Clone, Default)]
pub struct Dropper {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    /// Forever-ban set. A response id present here is never (re)admitted.
    banned: DashMap<String, ()>,
    /// Teardown context, late-bound by [`Dropper::wire`] from `setup`.
    ctx: OnceLock<TeardownCtx>,
}

struct TeardownCtx {
    sessions: Arc<SessionManager>,
    reverse_channel: Option<ReverseChannel>,
}

impl std::fmt::Debug for Dropper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dropper").finish_non_exhaustive()
    }
}

impl Dropper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forever-ban `response_id`, then tear down any live session for it.
    ///
    /// Bans FIRST so an `initialize` for the same id that runs
    /// concurrently observes the ban (its post-insert check then tears
    /// the session down itself). Teardown is idempotent, so a double
    /// teardown across the race is harmless.
    pub async fn drop(&self, response_id: String) {
        self.inner.banned.insert(response_id.clone(), ());
        if let Some(ctx) = self.inner.ctx.get() {
            teardown(&response_id, &ctx.sessions, ctx.reverse_channel.as_ref()).await;
        }
    }

    /// Whether `response_id` has been banned. Checked on every
    /// `initialize` (before reuse/connect, and again after insert).
    pub(crate) fn is_banned(&self, response_id: &str) -> bool {
        self.inner.banned.contains_key(response_id)
    }

    /// Wire the proxy's teardown context into the dropper. Called once by
    /// `setup` after it has built the session manager + reverse channel.
    /// Idempotent (first write wins).
    pub(crate) fn wire(
        &self,
        sessions: Arc<SessionManager>,
        reverse_channel: Option<ReverseChannel>,
    ) {
        let _ = self.inner.ctx.set(TeardownCtx {
            sessions,
            reverse_channel,
        });
    }
}

/// Forceful teardown of one response id's session.
///
/// Removes the session from the registry (dropping it closes the
/// proxy-side upstream connections), and — only if at least one upstream
/// is a `client_objectiveai_mcp` (ws) upstream — fires the reverse-channel
/// [`Drop`](objectiveai_sdk::client_objectiveai_mcp::server_request::Payload::Drop)
/// so the CLI tears down its side (connections + plugin subprocesses).
/// The `Drop` result is discarded. Idempotent: an absent id is a no-op,
/// and the CLI's `drop` handler is itself idempotent.
pub(crate) async fn teardown(
    response_id: &str,
    sessions: &SessionManager,
    reverse_channel: Option<&ReverseChannel>,
) {
    let Some(session) = sessions.remove(response_id) else {
        return;
    };
    let uses_objectiveai_mcp = session.connections.values().any(|u| u.is_ws());
    if uses_objectiveai_mcp {
        if let Some(rc) = reverse_channel {
            rc.drop_response(response_id.to_string()).await;
        }
    }
    // `session` (Arc<Session>) drops at end of scope → proxy-side upstream
    // connections close.
}
