//! Lock coordinates for the per-agent lockfiles.
//!
//! Two families, both under the per-state locks root and both held
//! through [`objectiveai_sdk::lockfile`] (contents are always empty —
//! these locks carry liveness, not data):
//!
//! - **Per-instance**: `<state>/locks/agents/instances/<…>` — the
//!   `agent_instance_hierarchy` is split on `/`; every segment but
//!   the last becomes a literal subdirectory, the last segment is the
//!   lockfile key (the SDK escapes the key; directory segments ride
//!   raw). A held instance lock ⇔ a live process owns that agent.
//! - **Per-tag**: `<state>/locks/agents/tags` keyed by the tag name —
//!   held while a spawn is materializing an un-upgraded (GROUPED)
//!   tag, released once the spawn claims its minted AIH lock.
//!
//! No `create_dir_all` here — the SDK's acquire functions create the
//! directory chain themselves.
//!
//! ## In-process gate
//!
//! [`objectiveai_sdk::lockfile`] is a cross-PROCESS mutex that is REENTRANT
//! in-process (its `HELD` map refcounts per process), so two concurrent
//! in-process tasks acquiring the SAME agent key both succeed reentrantly
//! instead of mutually excluding. [`AgentLock`] closes that gap: every agent
//! lock is taken through [`try_acquire`]/[`wait_acquire`] here, which lock a
//! per-key in-process [`tokio::sync::Mutex`] (the [`AgentLockMap`] on
//! [`crate::context::Context`]) FIRST, then the lockfile — and release the
//! lockfile claim FIRST, then the in-process guard LAST. The in-process lock
//! both precedes and succeeds the cross-process one.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::lockfile::LockClaim;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// `(lock_dir, key)` for an `agent_instance_hierarchy`.
pub fn agent_instance_lock(state_dir: &Path, agent_instance_hierarchy: &str) -> (PathBuf, String) {
    let mut dir = state_dir.join("locks").join("agents").join("instances");
    let mut segments = agent_instance_hierarchy.split('/').peekable();
    let mut key = String::new();
    while let Some(segment) = segments.next() {
        if segments.peek().is_some() {
            dir.push(segment);
        } else {
            key = segment.to_string();
        }
    }
    (dir, key)
}

/// `(lock_dir, key)` for an agent tag.
pub fn agent_tag_lock(state_dir: &Path, agent_tag: &str) -> (PathBuf, String) {
    (
        state_dir.join("locks").join("agents").join("tags"),
        agent_tag.to_string(),
    )
}

/// Per-key in-process gate for agent locks, keyed by the SAME `(dir, key)` the
/// lockfile uses. Lives on [`crate::context::Context`], shared across clones.
pub type AgentLockMap = DashMap<(PathBuf, String), Arc<Mutex<()>>>;

/// A held agent lock: the in-process [`Mutex`] guard PLUS the cross-process
/// lockfile [`LockClaim`]. Release order is enforced — lockfile claim first,
/// in-process guard last (the guard field is dropped after `claim` is taken).
pub struct AgentLock {
    /// `None` once released (explicitly or on drop) — distinguishes the
    /// explicit [`release`](Self::release) path from the [`Drop`] fallback.
    claim: Option<LockClaim>,
    /// Dropped LAST — frees the per-key in-process [`Mutex`].
    _guard: OwnedMutexGuard<()>,
}

impl AgentLock {
    /// Explicit release: lockfile claim first, then the guard drops at the end
    /// of scope. (Dropping a [`LockClaim`] does NOT release it — we must call
    /// `release`; the [`Drop`] impl below covers the implicit path.)
    pub fn release(mut self) -> std::io::Result<()> {
        match self.claim.take() {
            Some(claim) => claim.release(),
            None => Ok(()),
        }
    }
}

impl Drop for AgentLock {
    fn drop(&mut self) {
        // Scope-bound release (e.g. the registry's drop): claim first, then the
        // guard drops immediately after.
        if let Some(claim) = self.claim.take() {
            let _ = claim.release();
        }
    }
}

/// Get-or-insert the per-key in-process mutex, returning a cloned `Arc`. The
/// `DashMap` entry guard is dropped before returning — NEVER held across the
/// `.await` in the acquire fns.
fn mutex_for(map: &AgentLockMap, dir: &Path, key: &str) -> Arc<Mutex<()>> {
    map.entry((dir.to_path_buf(), key.to_string()))
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Acquire an agent lock NON-BLOCKING. Returns `None` if the key is busy
/// in-process (another task holds the guard) OR held cross-process. On the
/// cross-process miss the in-process guard is dropped before returning, so it
/// never lingers.
pub async fn try_acquire(map: &AgentLockMap, dir: &Path, key: &str) -> Option<AgentLock> {
    let guard = mutex_for(map, dir, key).try_lock_owned().ok()?;
    match objectiveai_sdk::lockfile::try_acquire(dir, key, "").await {
        Some(claim) => Some(AgentLock {
            claim: Some(claim),
            _guard: guard,
        }),
        None => None,
    }
}

/// Acquire an agent lock, BLOCKING at both layers: the in-process [`Mutex`]
/// first, then the cross-process lockfile.
pub async fn wait_acquire(
    map: &AgentLockMap,
    dir: &Path,
    key: &str,
) -> std::io::Result<AgentLock> {
    let guard = mutex_for(map, dir, key).lock_owned().await;
    let claim = objectiveai_sdk::lockfile::wait_acquire(dir, key, "").await?;
    Ok(AgentLock {
        claim: Some(claim),
        _guard: guard,
    })
}

/// The lock "family" of an agent target — the set of locks a live agent must
/// hold so that NONE of its tags can be relocated (`tags apply`) or have labs
/// detached (`laboratories detach`) while it is active.
#[derive(Clone)]
pub enum Family {
    /// A GROUPED tag: every tag in the group (they upgrade together).
    Group(i64),
    /// A bound tag or an AIH: the AIH lock + every tag bound to that AIH.
    Hierarchy(String),
}

/// An acquired lock [`Family`], partitioned for the registry.
pub struct AcquiredFamily {
    /// `(hierarchy, lock)` for the AIH lock — `None` for a GROUPED family.
    pub aih: Option<(String, AgentLock)>,
    /// The tag-lock family (group members, or the AIH's bound tags).
    pub tags: Vec<AgentLock>,
}

impl AcquiredFamily {
    /// Flatten to the raw lock list (AIH lock first, then tags) — for callers
    /// that hold or release the whole family rather than partitioning it into a
    /// registry (e.g. `agents message`, which releases the family before waking
    /// the agent with a fresh-competing child).
    pub fn into_locks(self) -> Vec<AgentLock> {
        let mut locks = Vec::with_capacity(self.tags.len() + 1);
        if let Some((_, aih)) = self.aih {
            locks.push(aih);
        }
        locks.extend(self.tags);
        locks
    }
}

/// Resolve + acquire a whole [`Family`] NON-BLOCKING, all-or-nothing, and
/// partition it for the registry (AIH lock split out from the tag locks).
/// `Ok(None)` if any member is busy. Used by spawn / deliver, which resolve the
/// family up front and acquire it in one shot.
pub async fn try_acquire_family(
    map: &AgentLockMap,
    pool: &crate::db::Pool,
    state_dir: &Path,
    family: Family,
) -> Result<Option<AcquiredFamily>, crate::error::Error> {
    // Keep the AIH string for the partition (family is consumed below).
    let aih = match &family {
        Family::Group(_) => None,
        Family::Hierarchy(h) => Some(h.clone()),
    };
    let coords = family_coords(pool, state_dir, family).await?;
    let Some(mut locks) = try_acquire_all(map, &coords).await else {
        return Ok(None);
    };
    // `family_coords` puts the AIH coord FIRST for a Hierarchy family.
    Ok(Some(match aih {
        Some(hierarchy) => AcquiredFamily {
            aih: Some((hierarchy, locks.remove(0))),
            tags: locks,
        },
        None => AcquiredFamily { aih: None, tags: locks },
    }))
}

/// Resolve a [`Family`] to its lock coordinates. `Group` → a tag lock per group
/// member; `Hierarchy` → the AIH instance lock FIRST, then a tag lock per bound
/// tag.
pub async fn family_coords(
    pool: &crate::db::Pool,
    state_dir: &Path,
    family: Family,
) -> Result<Vec<(PathBuf, String)>, crate::error::Error> {
    let mut coords = Vec::new();
    match family {
        Family::Group(tag_group) => {
            for tag in crate::db::tags::tags_for_group(pool, tag_group).await? {
                coords.push(agent_tag_lock(state_dir, &tag));
            }
        }
        Family::Hierarchy(agent_instance_hierarchy) => {
            coords.push(agent_instance_lock(state_dir, &agent_instance_hierarchy));
            for tag in crate::db::tags::tags_for_hierarchy(pool, &agent_instance_hierarchy).await? {
                coords.push(agent_tag_lock(state_dir, &tag));
            }
        }
    }
    Ok(coords)
}

/// Acquire EVERY coord NON-BLOCKING, all-or-nothing, concurrently. `None` if any
/// coord is busy (in-process guard taken OR cross-process held) — the ones that
/// were acquired drop here, releasing them. Deadlock-free: every leg is a
/// non-blocking `try`, so acquisition order never matters.
pub async fn try_acquire_all(
    map: &AgentLockMap,
    coords: &[(PathBuf, String)],
) -> Option<Vec<AgentLock>> {
    let got = futures::future::join_all(
        coords.iter().map(|(dir, key)| try_acquire(map, dir, key)),
    )
    .await;
    if got.iter().all(Option::is_some) {
        Some(got.into_iter().flatten().collect())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_instance_lock_splits_hierarchy() {
        let state = Path::new("state");
        let (dir, key) = agent_instance_lock(state, "root/child-1/leaf-2");
        assert_eq!(
            dir,
            Path::new("state")
                .join("locks")
                .join("agents")
                .join("instances")
                .join("root")
                .join("child-1"),
        );
        assert_eq!(key, "leaf-2");

        // Single-segment hierarchy: no subdirectories, whole string
        // is the key.
        let (dir, key) = agent_instance_lock(state, "UNKNOWN");
        assert_eq!(
            dir,
            Path::new("state").join("locks").join("agents").join("instances"),
        );
        assert_eq!(key, "UNKNOWN");
    }
}
