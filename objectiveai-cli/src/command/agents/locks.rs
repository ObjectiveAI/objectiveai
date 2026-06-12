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

use std::path::{Path, PathBuf};

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
