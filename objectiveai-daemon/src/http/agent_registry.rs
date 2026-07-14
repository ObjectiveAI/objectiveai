//! In-process exclusive claims keyed by `agent_instance_hierarchy`, over
//! the per-agent in-process mutex layout ([`crate::command::agents::locks`]).
//!
//! A `HashMap<hierarchy, AgentLock>` that attempts each hierarchy's
//! lock exactly once ([`AgentInstanceRegistry::observe`] is
//! best-effort — a failed try_acquire is silently dropped, meaning
//! another in-process task already holds that key). The registry can
//! carry a tag-lock family ([`AgentInstanceRegistry::hold_tag_claims`])
//! for spawns materializing a (GROUPED) tag; it is RETAINED for the
//! registry's lifetime, NOT dropped at the GROUPED→BOUND upgrade —
//! laboratories travel with a tag, so the tag must stay locked
//! (un-relocatable) for the agent's whole active life.
//!
//! Each [`AgentLock`] is an owned in-process mutex guard, so it releases
//! automatically when the registry (and thus the `HashMap`/`Vec` holding
//! it) drops — freeing the key and waking `wait_released` observers. An
//! in-process streaming spawn therefore frees its slots exactly when its
//! stream ends. No explicit release ceremony is needed.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::command::agents::locks::{AgentLock, AgentLockMap};
use crate::http::agents_routes::ActiveAgents;

pub struct AgentInstanceRegistry {
    state_dir: PathBuf,
    /// Shared per-key in-process gate (from [`crate::context::Context`]); every
    /// acquire here goes through it.
    agent_locks: Arc<AgentLockMap>,
    /// The resident daemon's live agent-status hub (from `Context`'s
    /// resident hubs), notified directly when an AIH goes active — the
    /// in-process replacement for the former `agents.sock`. `None` when
    /// this context isn't the resident daemon.
    active: Option<ActiveAgents>,
    open: HashMap<String, AgentLock>,
    /// Every hierarchy `observe` has ever tried — exactly one
    /// try_acquire per AIH per registry lifetime, success or not.
    attempted: HashSet<String>,
    /// The agent's tag-lock FAMILY: every tag in its GROUPED group, or every
    /// tag bound to its AIH. RETAINED for the registry's lifetime (released on
    /// drop) so NONE of the agent's tags — nor the laboratories that travel with
    /// them — can be relocated or detached while the agent is active.
    tag_claims: Vec<AgentLock>,
}

impl AgentInstanceRegistry {
    pub fn new(
        state_dir: PathBuf,
        agent_locks: Arc<AgentLockMap>,
        active: Option<ActiveAgents>,
    ) -> Self {
        Self {
            state_dir,
            agent_locks,
            active,
            open: HashMap::new(),
            attempted: HashSet::new(),
            tag_claims: Vec::new(),
        }
    }

    /// Hand the registry an AIH claim acquired up-front (the
    /// historic-spawn initial lock), so the mid-stream `observe` of
    /// the same hierarchy dedupes and the claim is released on drop
    /// with the rest.
    /// The AIH of a held per-instance claim, if any — the spawn
    /// error-logging eligibility check (an AIH lock is held) and its
    /// AIH source before the first chunk mints identity. A spawn holds
    /// at most one claim until `observe` runs, and `observe`'s AIH IS
    /// the identity — so "any key" is exact.
    pub fn aih(&self) -> Option<&str> {
        self.open.keys().next().map(String::as_str)
    }

    pub fn preseed(&mut self, hier: String, claim: AgentLock) {
        self.attempted.insert(hier.clone());
        self.announce_active(&hier);
        self.open.insert(hier, claim);
    }

    /// Hand the registry the agent's tag-lock family. Retained for the
    /// registry's lifetime (released on drop) — see the field doc.
    pub fn hold_tag_claims(&mut self, claims: Vec<AgentLock>) {
        self.tag_claims.extend(claims);
    }

    /// Idempotent, best-effort. The first time we see `hier`, try to
    /// acquire its instance lock; repeat calls are no-ops. Any
    /// failure (held by another process — or by THIS process via
    /// transferred handles, or the in-process guard busy) is silently
    /// dropped: the registry only tracks claims it really owns. The tag
    /// claim (if any) is RETAINED here — it is released only on drop, so
    /// the tag stays un-relocatable for the agent's whole active life.
    pub async fn observe(&mut self, hier: &str) {
        if !self.attempted.insert(hier.to_string()) {
            return;
        }
        let (dir, key) = crate::command::agents::locks::agent_instance_lock(&self.state_dir, hier);
        if let Some(claim) =
            crate::command::agents::locks::try_acquire(&self.agent_locks, &dir, &key).await
        {
            self.open.insert(hier.to_string(), claim);
            self.announce_active(hier);
        }
    }

    /// Best-effort, detached: tell the resident daemon's live agent-status
    /// hub this process just acquired `hier`'s instance lock, so its
    /// `/agents/instances/list` endpoint flips the agent active and starts
    /// watching the lock for release. Fire-and-forget — never blocks the
    /// acquire path, and an absent hub (not the resident daemon) is a
    /// silent no-op. The hub dedupes by AIH, so a re-announce (e.g. a child
    /// that inherited the lock) is harmless.
    fn announce_active(&self, hier: &str) {
        let Some(active) = self.active.clone() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let hier = hier.to_string();
        tokio::spawn(async move {
            active.activate(hier).await;
        });
    }

}

// No explicit `Drop`: dropping `open` + `tag_claims` drops every
// `AgentLock`, each of which frees its in-process mutex guard and fires
// its release `Notify`. With pure in-process mutexes, cross-key release
// ordering is irrelevant, so nothing needs sequencing here.
