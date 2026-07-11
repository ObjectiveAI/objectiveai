//! In-process lock coordinates + gate for agents and tags.
//!
//! Two key families, both addressed by a `(dir, key)` computed here. The
//! `dir` is only a map-key component now — NOTHING is written to disk:
//!
//! - **Per-instance**: key = the last `/`-segment of the
//!   `agent_instance_hierarchy`, dir =
//!   `<state>/locks/agents/instances/<parent segments>`. Held ⇔ a live
//!   in-process agent task owns that AIH.
//! - **Per-tag**: dir `<state>/locks/agents/tags`, key = the tag name —
//!   held while a spawn is materializing an un-upgraded (GROUPED) tag.
//!
//! Since the daemon is a single long-lived process, exclusion is a plain
//! per-key [`tokio::sync::Mutex`] in the [`AgentLockMap`] on
//! [`crate::context::Context`] (shared across every ctx clone — so it is
//! the single authoritative exclusion for every agent/tag key). Each entry
//! also carries a release [`Notify`], so observers can await a key going
//! free ([`wait_released`]) or probe it ([`try_held`]) WITHOUT acquiring it
//! — the in-process replacements for the former
//! `objectiveai_sdk::lockfile::{wait_released,try_held}`. The cross-process
//! lockfile layer these once sat on was removed when agents became
//! in-process tasks: one process needs no filesystem mutex.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

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

/// The per-key in-process lock entry: the exclusion [`Mutex`] plus a
/// release [`Notify`] fired when a held [`AgentLock`] drops, so observers
/// ([`wait_released`]) can wake and re-check.
pub struct LockEntry {
    mutex: Arc<Mutex<()>>,
    released: Notify,
}

/// Per-key in-process gate for agent locks, keyed by the SAME `(dir, key)`
/// [`agent_instance_lock`]/[`agent_tag_lock`] compute. Lives on
/// [`crate::context::Context`], shared across clones.
pub type AgentLockMap = DashMap<(PathBuf, String), Arc<LockEntry>>;

/// A held agent lock: an owned guard of the per-key in-process [`Mutex`].
/// Dropping it (explicitly via [`release`](Self::release) or at end of
/// scope) frees the mutex and then wakes every [`wait_released`] observer.
pub struct AgentLock {
    /// `Some` while held; taken on drop so the notify fires exactly once
    /// and the guard frees BEFORE the notify.
    guard: Option<OwnedMutexGuard<()>>,
    /// The key's entry — its `released` `Notify` fires after the guard frees.
    entry: Arc<LockEntry>,
}

impl AgentLock {
    /// Release now: free the mutex, then wake observers. Infallible — an
    /// in-process guard always releases on drop (no cross-process claim to
    /// fail). Consuming `self` runs [`Drop`].
    pub fn release(self) {}
}

impl Drop for AgentLock {
    fn drop(&mut self) {
        // Free the mutex FIRST (drop the guard), THEN notify — so a woken
        // `wait_released` re-check sees the key free.
        if let Some(guard) = self.guard.take() {
            drop(guard);
            self.entry.released.notify_waiters();
        }
    }
}

/// Get-or-insert the per-key [`LockEntry`], returning a cloned `Arc`. The
/// `DashMap` entry guard is dropped before returning — NEVER held across the
/// `.await` in the acquire fns.
fn entry_for(map: &AgentLockMap, dir: &Path, key: &str) -> Arc<LockEntry> {
    map.entry((dir.to_path_buf(), key.to_string()))
        .or_insert_with(|| {
            Arc::new(LockEntry {
                mutex: Arc::new(Mutex::new(())),
                released: Notify::new(),
            })
        })
        .clone()
}

/// Acquire an agent lock NON-BLOCKING. `None` if another in-process task
/// holds the key's mutex.
pub async fn try_acquire(map: &AgentLockMap, dir: &Path, key: &str) -> Option<AgentLock> {
    let entry = entry_for(map, dir, key);
    let guard = entry.mutex.clone().try_lock_owned().ok()?;
    Some(AgentLock { guard: Some(guard), entry })
}

/// Acquire an agent lock, BLOCKING until the key's in-process mutex is free.
pub async fn wait_acquire(map: &AgentLockMap, dir: &Path, key: &str) -> AgentLock {
    let entry = entry_for(map, dir, key);
    let guard = entry.mutex.clone().lock_owned().await;
    AgentLock { guard: Some(guard), entry }
}

/// Block until the key's lock is free (released, or never held). The
/// in-process replacement for `objectiveai_sdk::lockfile::wait_released`:
/// register for the NEXT release, then re-check — the enable-before-check
/// order closes the wake gap, and the `try_lock` handles the already-free
/// case (observation only — the guard drops immediately).
pub async fn wait_released(map: &AgentLockMap, dir: &Path, key: &str) {
    let entry = entry_for(map, dir, key);
    loop {
        let notified = entry.released.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if entry.mutex.try_lock().is_ok() {
            return;
        }
        notified.await;
    }
}

/// Whether the key is currently held by some in-process task. The
/// in-process replacement for `objectiveai_sdk::lockfile::try_held`
/// (observation only — the momentary `try_lock` guard drops at once).
pub fn try_held(map: &AgentLockMap, dir: &Path, key: &str) -> bool {
    entry_for(map, dir, key).mutex.try_lock().is_err()
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
