//! Browser profiles: where a persistent browser tab's cookies,
//! localStorage and cache live, and who is allowed to hold one.
//!
//! **A profile directory MUST be an immediate child of the
//! process-global root.** Chromium's `ProfileManager` refuses a nested
//! `cache_path` outright ("Cannot create profile at path ..."), and the
//! refusal is silent from the embedder's side: the browser still opens,
//! still browses, and simply never writes a cookie store — so sign-ins
//! evaporate on close with nothing in the logs but CEF's own
//! `cef-debug.log`. Hence [`ProfileRoot::profile`] returns ONE flat
//! hashed segment, never a `<identity>/<key>` path.
//!
//! The hash is over the owning identity and the tab's `state` key, each
//! fed with an explicit length prefix so `("a", "bc")` and `("ab", "c")`
//! cannot collide into the same directory. An opaque directory name is
//! hard to explain later, so each profile keeps a plaintext
//! `identity.json` breadcrumb naming what minted it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use objectiveai_sdk::lockfile::LockClaim;

/// The process-global browsers root: `<viewer_dir>/browsers/`. Managed
/// Tauri state, set once at startup — CEF's `root_cache_path` is locked
/// for the process lifetime the moment `initialize` runs, so this can
/// never be re-pointed either. Every relocation orphans every profile
/// on disk, so it is chosen once and left alone.
pub struct ProfileRoot {
    root: PathBuf,
    /// `<objectiveai_dir>/bin/locks` — where the claim on a live
    /// profile lives, beside every other machine-wide install lock.
    locks: PathBuf,
    /// Profiles claimed by THIS process.
    ///
    /// The lockfile alone is not enough: `try_acquire` is deliberately
    /// REENTRANT — a second acquire of a key this process already holds
    /// succeeds and refcounts, so that a process never deadlocks
    /// against its own lock. That is right for install locks and wrong
    /// here, where the second claimant is a second browser about to
    /// write the same SQLite store. This set is the in-process half of
    /// the exclusion; the lockfile is the cross-process half.
    held: Arc<Mutex<HashSet<String>>>,
}

/// A claimed persistent profile: the directory to hand CEF, and the
/// lock proving no other browser (in this process or any other) is
/// writing it. Dropping the claim does NOT release it — the browser
/// tab's close does, explicitly, via [`Profile::release`].
pub struct Profile {
    pub dir: PathBuf,
    segment: String,
    held: Arc<Mutex<HashSet<String>>>,
    claim: LockClaim,
}

impl Profile {
    /// Give up the claim so the same identity + state key can open a
    /// browser tab again. Called once the browser is fully closed —
    /// releasing earlier would let a second tab attach to a store
    /// Chromium is still flushing.
    pub fn release(self) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(&self.segment);
        }
        let _ = self.claim.release();
    }
}

impl ProfileRoot {
    pub fn new(viewer_dir: &Path, bin_dir: &Path) -> Self {
        Self {
            root: viewer_dir.join("browsers"),
            locks: bin_dir.join("locks"),
            held: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// The root CEF is initialized with. Every profile is an immediate
    /// child of it.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Claim the profile for `identity` + `state`, creating it if this
    /// is its first use.
    ///
    /// Fails — immediately, never blocking — when another browser
    /// already holds it. Waiting would be worse than useless: the
    /// holder is a browser tab someone has open, so the wait would be
    /// unbounded, and proceeding anyway would corrupt a SQLite store
    /// Chromium believes it owns exclusively.
    pub async fn claim(&self, identity: &str, state: &str) -> Result<Profile, String> {
        let segment = self.segment(identity, state);
        let taken = format!(
            "browser profile {state:?} is already open — a persistent browser \
             profile can only be driven by one browser at a time"
        );
        // In-process first, and BEFORE any disk work: winning this
        // insert is what makes the whole claim exclusive (see
        // [`ProfileRoot::held`]).
        match self.held.lock() {
            Ok(mut held) => {
                if !held.insert(segment.clone()) {
                    return Err(taken);
                }
            }
            Err(_) => return Err("browser profile registry poisoned".to_string()),
        }
        // From here every failure must hand the slot back.
        let release_slot = || {
            if let Ok(mut held) = self.held.lock() {
                held.remove(&segment);
            }
        };
        let dir = self.root.join(&segment);
        if let Err(e) = tokio::fs::create_dir_all(&dir).await {
            release_slot();
            return Err(format!("create browser profile {dir:?}: {e}"));
        }

        // The breadcrumb: written every claim (cheap, and it heals a
        // profile whose breadcrumb was lost) so an opaque directory
        // name is always explicable.
        let breadcrumb = serde_json::json!({
            "identity": identity,
            "state": state,
        });
        let _ = tokio::fs::write(
            dir.join("identity.json"),
            serde_json::to_vec_pretty(&breadcrumb).unwrap_or_default(),
        )
        .await;

        // The cross-process half: another VIEWER holding this profile.
        let claim = objectiveai_sdk::lockfile::try_acquire(
            &self.locks,
            &format!("cef-profile-{segment}"),
            &format!("pid {}", std::process::id()),
        )
        .await;
        let Some(claim) = claim else {
            release_slot();
            return Err(taken);
        };

        Ok(Profile {
            dir,
            segment,
            held: self.held.clone(),
            claim,
        })
    }

    /// The flat directory name for one (identity, state) pair.
    ///
    /// The root is folded in as well: two ObjectiveAI states have
    /// separate `browsers/` roots but share the machine-wide lock
    /// directory, and without the root in the hash the same plugin
    /// under the same state key would collide on the LOCK across
    /// states — one of them failing to open for no visible reason.
    fn segment(&self, identity: &str, state: &str) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        for part in [
            self.root.to_string_lossy().as_ref(),
            identity,
            state,
        ] {
            // Length-prefixed: without it ("a","bc") and ("ab","c")
            // hash the same bytes and share a profile.
            hasher.write(&(part.len() as u64).to_le_bytes());
            hasher.write(part.as_bytes());
        }
        format!("{:032x}", hasher.finish_128())
    }
}
