//! Process-owned exclusive claims keyed by
//! `agent_instance_hierarchy`, backed by
//! [`objectiveai_sdk::lockfile`] at the per-agent lock layout
//! ([`crate::command::agents::locks`]).
//!
//! A `HashMap<hierarchy, AgentLock>` that attempts each hierarchy's
//! lock exactly once ([`AgentInstanceRegistry::observe`] is
//! best-effort — a failed try_acquire is silently dropped, which
//! also covers the case where this process already holds the lock
//! via handles a parent transferred in). The registry can carry one
//! tag claim ([`AgentInstanceRegistry::hold_tag_claim`]) for spawns
//! materializing a (GROUPED) tag; it is RETAINED for the registry's
//! lifetime (released on drop with the rest), NOT dropped at the
//! GROUPED→BOUND upgrade — laboratories travel with a tag, so the tag
//! must stay locked (un-relocatable) for the agent's whole active life.
//!
//! SDK claims do NOT release on drop (their handles are leaked on
//! purpose so a lock normally lives until process death). The
//! registry restores scope-bound semantics: its `Drop` explicitly
//! releases every claim it owns, so an in-process streaming spawn
//! frees its slots when its stream ends. Transferred-in locks have
//! no claim object here and die with the process — by design, since
//! the process IS the spawn.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::command::agents::locks::{AgentLock, AgentLockMap};

pub struct AgentInstanceRegistry {
    state_dir: PathBuf,
    /// Shared per-key in-process gate (from [`crate::context::Context`]); every
    /// acquire here goes through it.
    agent_locks: Arc<AgentLockMap>,
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
    pub fn new(state_dir: PathBuf, agent_locks: Arc<AgentLockMap>) -> Self {
        Self {
            state_dir,
            agent_locks,
            open: HashMap::new(),
            attempted: HashSet::new(),
            tag_claims: Vec::new(),
        }
    }

    /// Hand the registry an AIH claim acquired up-front (the
    /// historic-spawn initial lock), so the mid-stream `observe` of
    /// the same hierarchy dedupes and the claim is released on drop
    /// with the rest.
    pub fn preseed(&mut self, hier: String, claim: AgentLock) {
        self.attempted.insert(hier.clone());
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
        }
    }

    /// Release `hier`'s claim immediately — another process can then
    /// acquire the slot. No-op if `hier` was never observed or never
    /// produced a successful claim.
    pub fn destroy(&mut self, hier: &str) {
        if let Some(claim) = self.open.remove(hier) {
            let _ = claim.release();
        }
    }
}

impl Drop for AgentInstanceRegistry {
    fn drop(&mut self) {
        // AIH claims first, then the tag family — each AgentLock releases its
        // lockfile claim, then drops its in-process guard.
        for (_, claim) in self.open.drain() {
            let _ = claim.release();
        }
        for claim in self.tag_claims.drain(..) {
            let _ = claim.release();
        }
    }
}
