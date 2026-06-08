//! Process-owned exclusive claim files keyed by
//! `agent_instance_hierarchy`.
//!
//! Thin index over [`crate::websockets::lock_file`]'s OS-managed
//! claim primitive: a `HashMap<hierarchy, LockClaim>` that
//! dedupes repeat `observe` calls and lets the owner explicitly
//! release a single hierarchy mid-stream via `destroy`.
//!
//! See [`crate::websockets::lock_file`] for platform semantics —
//! Windows uses `FILE_FLAG_DELETE_ON_CLOSE` (file existence ⇔
//! liveness); Unix uses persistent file + `flock` (lock state ⇔
//! liveness).
//!
//! Every operation is best-effort: [`observe`] returns `()` and
//! silently swallows IO errors. The registry only tracks claims it
//! actually owns.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::websockets::lock_file::{self, LockClaim};

pub struct AgentInstanceRegistry {
    root: PathBuf,
    open: HashMap<String, LockClaim>,
}

impl AgentInstanceRegistry {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            open: HashMap::new(),
        })
    }

    /// Path the registry uses for `hier`. Exposed so callers
    /// (notably [`crate::command::agents::message`])
    /// can subscribe to release / acquisition events on the same
    /// physical lock file without going through the registry's
    /// owned-handle API.
    pub fn path_for(&self, hier: &str) -> PathBuf {
        self.root.join(hier.replace('/', "_"))
    }

    /// Idempotent, best-effort. The first time we see `hier`, try
    /// to acquire its lock file. Repeat calls are no-ops. Any
    /// failure (file already claimed live, illegal chars, ENOSPC,
    /// …) is silently dropped — the registry only tracks claims
    /// it really owns.
    pub fn observe(&mut self, hier: &str) {
        if self.open.contains_key(hier) {
            return;
        }
        let path = self.path_for(hier);
        if let Some(claim) = lock_file::try_acquire(&path) {
            self.open.insert(hier.to_string(), claim);
        }
    }

    /// Release the claim immediately. On Windows the file is gone
    /// from disk (DELETE_ON_CLOSE fires when the underlying
    /// handle drops). On Unix the file persists but the `flock`
    /// is released — another process can detect the unlocked
    /// state and reclaim the hierarchy. No-op if `hier` was never
    /// observed or never produced a successful claim.
    pub fn destroy(&mut self, hier: &str) {
        self.open.remove(hier);
    }
}
