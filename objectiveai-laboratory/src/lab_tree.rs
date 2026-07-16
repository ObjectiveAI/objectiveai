//! One laboratory's unified filetree as its SOURCE SET — the clean
//! abstraction that makes partial trees unrepresentable.
//!
//! A laboratory's tree is fed by N+1 sources: the CONTAINER's
//! `/filetree` stream plus one source per MOUNT (watched natively by
//! [`crate::mount_watch`]). The completeness invariant lives here, in
//! the type, not in call-site choreography:
//!
//! - The source set is declared at registration, BEFORE any data
//!   flows ([`LabTree::new`] / [`LabTree::merge_sources`]).
//! - A tree is **complete** exactly when the container has delivered
//!   its first snapshot AND every mount source has delivered its
//!   first walk ([`LabTree::complete`]).
//! - [`LabTree::compose`] — the ONLY way to obtain a whole tree —
//!   returns `None` unless complete. There is no API that yields a
//!   container-only or mount-less tree, so the first snapshot the
//!   host ever emits for a laboratory is complete and total by
//!   construction, and every re-snapshot (container reconnect, mount
//!   resync) is too.
//!
//! Emission policy (owned by `HostServer`'s `source_*` ingestion
//! methods, all under `attach_lock`): nothing is emitted for an
//! incomplete tree; the source delivery that completes the set emits
//! one composed `Snapshot`; afterwards deltas pass through and any
//! source re-delivery emits a fresh composed `Snapshot` — so a mount
//! can never appear empty, pop in late, or flicker out across
//! container reconnects.
//!
//! Mount data is NOT copied per laboratory: compose reads each mount
//! watch's cached tree live (a sync leaf lock), so two labs mounting
//! the same directory share one tree — and a torn-down watch's cached
//! tree keeps serving the frozen view through the `Arc` until a
//! restarted lab's fresh walk re-delivers ([`LabTree::mount_delivered`]
//! swaps the `Arc` only at delivery time).

use std::collections::HashMap;
use std::sync::Arc;

use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};

use crate::mount_watch::MountWatch;

/// One laboratory's tree-as-source-set. Lives in `HostServer.filetree`
/// (created at registration, removed at delete); every mutation
/// happens under `attach_lock`.
pub struct LabTree {
    /// The container's tree — `None` until its first Snapshot.
    container: Option<Vec<FileTreeNode>>,
    /// One entry per mount, keyed by CONTAINER mountpoint components.
    mounts: HashMap<Vec<String>, MountSource>,
}

/// A mount source: the shared watch whose cached tree compose reads
/// live, and whether it has delivered its first walk to THIS lab.
struct MountSource {
    watch: Arc<MountWatch>,
    delivered: bool,
}

impl LabTree {
    /// A fresh tree with every source pending.
    pub fn new(mounts: impl IntoIterator<Item = (Vec<String>, Arc<MountWatch>)>) -> Self {
        Self {
            container: None,
            mounts: mounts
                .into_iter()
                .map(|(mountpoint, watch)| {
                    (
                        mountpoint,
                        MountSource {
                            watch,
                            delivered: false,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Re-registration (lab restart): add any missing mount sources as
    /// pending; EXISTING sources are left untouched — old watch `Arc`,
    /// old `delivered` flag — so a frozen view stays complete until
    /// the restarted mount's fresh walk re-delivers (which is when
    /// [`Self::mount_delivered`] swaps the `Arc`).
    pub fn merge_sources(
        &mut self,
        mounts: impl IntoIterator<Item = (Vec<String>, Arc<MountWatch>)>,
    ) {
        for (mountpoint, watch) in mounts {
            self.mounts.entry(mountpoint).or_insert(MountSource {
                watch,
                delivered: false,
            });
        }
    }

    /// The container's first (or reconnect) snapshot.
    pub fn set_container(&mut self, children: Vec<FileTreeNode>) {
        self.container = Some(children);
    }

    /// Fold a container delta. `false` (a drop) when no container
    /// snapshot exists yet — a delta without its base is meaningless.
    pub fn fold_container(&mut self, event: FileTreeEvent) -> bool {
        match &mut self.container {
            Some(root) => {
                event.apply(root);
                true
            }
            None => false,
        }
    }

    /// A mount source delivered (initial walk, resync re-walk, or a
    /// restarted watch's fresh walk — the `Arc` swap happens HERE, at
    /// delivery, never at registration). `false` when the mountpoint
    /// was never a registered source (stale subscription — dropped).
    pub fn mount_delivered(&mut self, mountpoint: &[String], watch: &Arc<MountWatch>) -> bool {
        match self.mounts.get_mut(mountpoint) {
            Some(source) => {
                source.watch = Arc::clone(watch);
                source.delivered = true;
                true
            }
            None => false,
        }
    }

    /// Whether this mountpoint is a delivered source — the gate for
    /// passing its deltas through.
    pub fn mount_ready(&self, mountpoint: &[String]) -> bool {
        self.mounts
            .get(mountpoint)
            .is_some_and(|source| source.delivered)
    }

    /// Complete = every source has delivered.
    pub fn complete(&self) -> bool {
        self.container.is_some() && self.mounts.values().all(|source| source.delivered)
    }

    /// The whole unified tree — `None` unless [`Self::complete`]. The
    /// container's tree with every mount's live cached tree grafted at
    /// its mountpoint (missing middles synthesized by the shared fold;
    /// the graft root is renamed to the mountpoint's basename). A file
    /// mount whose file is currently gone contributes nothing — the
    /// source is still delivered, absence is its truthful state.
    pub fn compose(&self) -> Option<Vec<FileTreeNode>> {
        if !self.complete() {
            return None;
        }
        let mut root = self.container.clone()?;
        for (mountpoint, source) in &self.mounts {
            let Some(node) = source.watch.snapshot_node(mountpoint) else {
                continue;
            };
            FileTreeEvent::Upserted {
                path: mountpoint.clone(),
                node,
            }
            .apply(&mut root);
        }
        Some(root)
    }
}
