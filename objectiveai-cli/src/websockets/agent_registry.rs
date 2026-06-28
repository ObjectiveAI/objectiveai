//! Process-owned exclusive claims keyed by
//! `agent_instance_hierarchy`, backed by
//! [`objectiveai_sdk::lockfile`] at the per-agent lock layout
//! ([`crate::command::agents::locks`]).
//!
//! A `HashMap<hierarchy, LockClaim>` that attempts each hierarchy's
//! lock exactly once ([`AgentInstanceRegistry::observe`] is
//! best-effort — a failed try_acquire is silently dropped, which
//! also covers the case where this process already holds the lock
//! via handles a parent transferred in). The registry can carry one
//! tag claim ([`AgentInstanceRegistry::hold_tag_claim`]) for spawns
//! materializing an un-upgraded tag; the FIRST successful AIH claim
//! releases it — once the minted hierarchy's lock is held, the tag
//! lock's job is done.
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
    /// Tag claim guarding an un-upgraded (GROUPED) tag spawn.
    /// Released on the first successful AIH claim, held to the end
    /// otherwise.
    tag_claim: Option<AgentLock>,
}

impl AgentInstanceRegistry {
    pub fn new(state_dir: PathBuf, agent_locks: Arc<AgentLockMap>) -> Self {
        Self {
            state_dir,
            agent_locks,
            open: HashMap::new(),
            attempted: HashSet::new(),
            tag_claim: None,
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

    /// Hand the registry the un-upgraded tag's claim. Released by
    /// the first successful AIH claim, or on drop.
    pub fn hold_tag_claim(&mut self, claim: AgentLock) {
        self.tag_claim = Some(claim);
    }

    /// Idempotent, best-effort. The first time we see `hier`, try to
    /// acquire its instance lock; repeat calls are no-ops. Any
    /// failure (held by another process — or by THIS process via
    /// transferred handles, or the in-process guard busy) is silently
    /// dropped: the registry only tracks claims it really owns. On the
    /// first success, the held tag claim (if any) is released.
    pub async fn observe(&mut self, hier: &str) {
        if !self.attempted.insert(hier.to_string()) {
            return;
        }
        let (dir, key) = crate::command::agents::locks::agent_instance_lock(&self.state_dir, hier);
        if let Some(claim) =
            crate::command::agents::locks::try_acquire(&self.agent_locks, &dir, &key).await
        {
            self.open.insert(hier.to_string(), claim);
            // The minted hierarchy is now guarded directly — the tag
            // lock has served its purpose.
            if let Some(tag_claim) = self.tag_claim.take() {
                let _ = tag_claim.release();
            }
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
        // AIH claims first, then the tag claim — each AgentLock releases its
        // lockfile claim, then drops its in-process guard.
        for (_, claim) in self.open.drain() {
            let _ = claim.release();
        }
        if let Some(claim) = self.tag_claim.take() {
            let _ = claim.release();
        }
    }
}
