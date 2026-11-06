//! A run's life on its log: the one writer of `deployed`, `active`,
//! `inactive`, `undeployed` and the interrupted answers, for agents
//! and tools alike.
//!
//! Every run's log — an agent's, a tool's — is live as one [`RunLog`],
//! and its lock holds where the log stands: a [`Lifecycle`]. Every
//! append to a run's log, a chunk or an error as much as a lifecycle
//! item, is made under that lock, and the lifecycle items are written
//! only here, against what it holds. The rules:
//!
//! 1. **Deployed only from undeployed.** [`deployed`] keeps
//!    `deployed` only when the log stands undeployed, and the log
//!    then stands deployed under the run's `epoch` — a number fresh
//!    from the daemon for every run. A log still standing deployed
//!    under another run, whose end was never kept, is closed first as
//!    rule 4 closes one, with no reason.
//! 2. **Active and inactive only while deployed, only on a change.**
//!    [`active`] keeps `active` or `inactive` only while the log stands
//!    deployed under the caller's epoch, and only when it changes what
//!    the log holds. A run that is not the one deployed writes nothing:
//!    no `active` before `deployed`, and none from an old run.
//! 2a. **A cut is no ceasing.** [`cut`] marks the deployed run cut: an
//!    exchange for it ended because a scope or a connection under it
//!    went, which is no clean completion. While it is cut, [`active`]
//!    keeps no `inactive`, and the log holds the run active: if the run
//!    is going down, its `undeployed` keeps the `inactive` itself, with
//!    `interrupted` true. [`uncut`] clears the mark once the run is
//!    proven alive, and keeps `inactive` then only when the run, asked
//!    under the lock, is idle.
//! 3. **Asks open and close the work.** [`begin`] keeps an ask only
//!    while the log stands deployed under the caller's epoch, after
//!    `active` when none was open, and holds it open; it is not kept
//!    otherwise, and the ask goes on unlogged. [`end`] keeps the ask's
//!    answer only while that run is deployed and the ask open, closes
//!    it, and keeps `inactive` after the answer that leaves none open.
//!    An answer that is the run going down — [`Answer::Interrupted`] —
//!    keeps nothing and leaves the ask open, for rule 4 to close.
//! 3a. **Unless the run lives on.** An ask answered
//!    [`Answer::Interrupted`] is marked interrupted. Once the run is
//!    proven alive after all — a later ask of it answered and kept by
//!    [`end`], or its cut cleared by [`uncut`] — every ask so marked is
//!    answered as interrupted, in the order they were asked, and
//!    closed, before anything else is kept: a run that survives holds
//!    no ask open for good.
//! 4. **Undeployed closes everything, once.** [`undeployed`] keeps,
//!    only while the log stands deployed under the caller's epoch and
//!    in this order: each ask still open answered as interrupted, in
//!    the order they were asked; `inactive`, when the log holds the
//!    run active; then `undeployed`, with the run's reason and
//!    `interrupted` true exactly when the run was active. The log then
//!    stands undeployed, and nothing of that epoch is ever kept again:
//!    no `inactive` after `undeployed`.
//! 5. **Forgotten.** [`forget`] makes the log stand forgotten — its
//!    record deleted — and nothing is kept on it again, by anyone.
//! 6. **Uncancellable.** Every write here runs in a task of its own,
//!    holding the lock for its whole length, and is waited for: a
//!    caller cancelled midway — a task aborted at a run's end, a
//!    caller gone — neither tears an append nor leaves the log half
//!    moved. A failed append fails nothing it records: the write is
//!    let go, and the state moves as though it had landed.
//!
//! [`End`] is how a run is ending, said once by whoever ends it, and
//! [`End::reason`] is what its `undeployed` carries.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use diverge_sdk::daemon::agent_log;
use diverge_sdk::daemon::lifecycle::{self as items, Provider, Reason};
use diverge_sdk::daemon::tool_log;
use diverge_sdk::shared::Identity;
use tokio::sync::{Mutex, watch};

use super::ToolKey;
use crate::logs::{self, Book, Entry};
use crate::store::AgentId;

/// Where a run's log stands: what its lock holds.
#[derive(Debug)]
pub enum Lifecycle {
    /// No run is deployed: before the first, and after every
    /// `undeployed`.
    Undeployed,
    /// The run of `epoch` is deployed on `provider`.
    Deployed {
        /// The run's number.
        epoch: u64,
        /// Where it is deployed: what its `undeployed` names.
        provider: Identity,
        /// Whether the log holds it working: after an `active`, before
        /// the `inactive` that answers it.
        active: bool,
        /// Whether an exchange for it was cut short and the run is not
        /// yet proven alive since: rule 2a.
        cut: bool,
        /// The asks kept and not yet answered, by `logs_index`; a tool's
        /// only.
        open: BTreeMap<u64, Ask>,
        /// The open asks answered [`Answer::Interrupted`], by
        /// `logs_index`: rule 3a.
        interrupted: BTreeSet<u64>,
    },
    /// The record is deleted: nothing is kept again.
    Forgotten,
}

/// Which kind an open ask is, for its interrupted answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// A tool call.
    Call,
    /// A resource read.
    Read,
}

/// The answer to an ask, as [`end`] takes it.
pub enum Answer<I> {
    /// The tool answered, or failed to in words of its own: kept.
    Kept(I),
    /// The run's scope or its connection ended under the ask: nothing
    /// is kept now; the run's `undeployed` answers it — rule 3.
    Interrupted,
}

/// How a run is ending, said once, by whoever ends it first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Asked to end: gone idle, its record deleted, retired, its
    /// owner's run over. Its `undeployed` carries no reason.
    Expected,
    /// Ended without anyone asking: its `undeployed` carries the
    /// reason.
    Unexpected(Reason),
}

impl End {
    /// What the run's `undeployed` carries.
    pub fn reason(self) -> Option<Reason> {
        match self {
            End::Expected => None,
            End::Unexpected(reason) => Some(reason),
        }
    }
}

/// One run's log as it is live — an agent's or a tool's: the lock every
/// append is made under, holding where the log stands, and the latest
/// index as a watch, so a reader waiting on more learns of every
/// append. The sender gone — the record deleted — ends every watch.
#[derive(Debug)]
pub struct RunLog {
    /// Taken for the length of one write, holding where the log stands.
    pub lock: Mutex<Lifecycle>,
    /// The `logs_index` of the latest item appended since the start;
    /// `0` before any. What a reader waits on.
    pub latest: watch::Sender<u64>,
}

impl RunLog {
    /// A log that stands undeployed, nothing appended since the start.
    pub fn new() -> Self {
        RunLog { lock: Mutex::new(Lifecycle::Undeployed), latest: watch::channel(0).0 }
    }

    /// A log that stands forgotten: what a deleted record's writers get.
    pub fn forgotten() -> Self {
        RunLog { lock: Mutex::new(Lifecycle::Forgotten), latest: watch::channel(0).0 }
    }
}

impl Default for RunLog {
    fn default() -> Self {
        RunLog::new()
    }
}

/// A book a run's log is kept in: how each lifecycle item is written
/// in it. An agent's and a tool's write the same JSON.
pub trait RunBook: Book + Send + Sync + 'static {
    /// `deployed`.
    fn deployed(deployed: items::Deployed) -> <Self::Entry as Entry>::Item;
    /// `active`.
    fn active() -> <Self::Entry as Entry>::Item;
    /// `inactive`.
    fn inactive() -> <Self::Entry as Entry>::Item;
    /// `undeployed`.
    fn undeployed(undeployed: items::Undeployed) -> <Self::Entry as Entry>::Item;
    /// The interrupted answer to the open ask at `index`, when the book
    /// keeps asks.
    fn interrupted(index: u64, ask: Ask) -> Option<<Self::Entry as Entry>::Item>;
}

impl RunBook for AgentId {
    fn deployed(deployed: items::Deployed) -> agent_log::Item {
        agent_log::Item::Deployed(agent_log::Deployed { r#type: Default::default(), deployed })
    }

    fn active() -> agent_log::Item {
        agent_log::Item::Active(agent_log::Active { r#type: Default::default() })
    }

    fn inactive() -> agent_log::Item {
        agent_log::Item::Inactive(agent_log::Inactive { r#type: Default::default() })
    }

    fn undeployed(undeployed: items::Undeployed) -> agent_log::Item {
        agent_log::Item::Undeployed(agent_log::Undeployed { r#type: Default::default(), undeployed })
    }

    /// An agent's log keeps no asks.
    fn interrupted(_index: u64, _ask: Ask) -> Option<agent_log::Item> {
        None
    }
}

impl RunBook for ToolKey {
    fn deployed(deployed: items::Deployed) -> tool_log::Item {
        tool_log::Item::Deployed(deployed)
    }

    fn active() -> tool_log::Item {
        tool_log::Item::Active
    }

    fn inactive() -> tool_log::Item {
        tool_log::Item::Inactive
    }

    fn undeployed(undeployed: items::Undeployed) -> tool_log::Item {
        tool_log::Item::Undeployed(undeployed)
    }

    fn interrupted(index: u64, ask: Ask) -> Option<tool_log::Item> {
        Some(match ask {
            Ask::Call => tool_log::Item::CallInterrupted(tool_log::CallInterrupted { call: index }),
            Ask::Read => tool_log::Item::ReadInterrupted(tool_log::ReadInterrupted { read: index }),
        })
    }
}

/// Rule 6: `work` in a task of its own, waited for. A task that
/// panicked answers `None`.
async fn whole<T: Send + 'static>(work: impl Future<Output = T> + Send + 'static) -> Option<T> {
    tokio::spawn(work).await.ok()
}

/// One item appended under the held lock, the watchers told; its index,
/// or `None` when it could not be written.
async fn write<B: RunBook>(root: &PathBuf, book: B, latest: &watch::Sender<u64>, item: <B::Entry as Entry>::Item) -> Option<u64> {
    let wrapper = logs::append(root, book, item).await.ok()?;
    let index = wrapper.logs_index();
    latest.send_replace(index);
    Some(index)
}

/// Rule 4's closing, under the held lock: every open ask answered as
/// interrupted, `inactive` when active, `undeployed`; the log left
/// undeployed. Whether the run was active.
async fn close<B: RunBook>(root: &PathBuf, book: B, log: &RunLog, state: &mut Lifecycle, reason: Option<Reason>) -> bool {
    let Lifecycle::Deployed { provider, active, open, .. } = std::mem::replace(state, Lifecycle::Undeployed) else {
        return false;
    };
    for (index, ask) in open {
        if let Some(item) = B::interrupted(index, ask) {
            write(root, book, &log.latest, item).await;
        }
    }
    if active {
        write(root, book, &log.latest, B::inactive()).await;
    }
    let undeployed = items::Undeployed { provider: Provider { identity: provider }, reason, interrupted: active };
    write(root, book, &log.latest, B::undeployed(undeployed)).await;
    active
}

/// Rule 1: the run of `epoch` deployed on `provider`.
pub async fn deployed<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, epoch: u64, provider: Identity) {
    whole(async move {
        let mut state = log.lock.lock().await;
        match &*state {
            Lifecycle::Forgotten => return,
            Lifecycle::Deployed { .. } => {
                close(&root, book, &log, &mut state, None).await;
            }
            Lifecycle::Undeployed => {}
        }
        let deployed = items::Deployed { provider: Provider { identity: provider.clone() } };
        write(&root, book, &log.latest, B::deployed(deployed)).await;
        *state = Lifecycle::Deployed { epoch, provider, active: false, cut: false, open: BTreeMap::new(), interrupted: BTreeSet::new() };
    })
    .await;
}

/// Rule 2: the run of `epoch` working, or not. Whether the log moved.
pub async fn active<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, epoch: u64, working: bool) -> bool {
    whole(async move {
        let mut state = log.lock.lock().await;
        let Lifecycle::Deployed { epoch: deployed, active, cut, .. } = &mut *state else {
            return false;
        };
        if *deployed != epoch || *active == working || (*cut && !working) {
            return false;
        }
        *active = working;
        let item = if working { B::active() } else { B::inactive() };
        write(&root, book, &log.latest, item).await;
        true
    })
    .await
    .unwrap_or(false)
}

/// Rule 2a: the run of `epoch` cut — an exchange for it ended because a
/// scope or a connection under it went.
pub async fn cut(log: Arc<RunLog>, epoch: u64) {
    whole(async move {
        if let Lifecycle::Deployed { epoch: deployed, cut, .. } = &mut *log.lock.lock().await
            && *deployed == epoch
        {
            *cut = true;
        }
    })
    .await;
}

/// Rule 3a, under the held lock: every open ask marked interrupted
/// answered as interrupted, in the order asked, and closed.
async fn settle<B: RunBook>(root: &PathBuf, book: B, latest: &watch::Sender<u64>, open: &mut BTreeMap<u64, Ask>, interrupted: &mut BTreeSet<u64>) {
    for index in std::mem::take(interrupted) {
        if let Some(ask) = open.remove(&index)
            && let Some(item) = B::interrupted(index, ask)
        {
            write(root, book, latest, item).await;
        }
    }
}

/// Rules 2a and 3a: the run of `epoch` proven alive since its cut: the
/// mark cleared, every interrupted ask closed, and `inactive` kept when
/// no ask is open and `working`, asked under the lock, says it is idle.
/// Whether the log moved.
pub async fn uncut<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, epoch: u64, working: impl FnOnce() -> bool + Send + 'static) -> bool {
    whole(async move {
        let mut state = log.lock.lock().await;
        let Lifecycle::Deployed { epoch: deployed, active, cut, open, interrupted, .. } = &mut *state else {
            return false;
        };
        if *deployed != epoch || !*cut {
            return false;
        }
        *cut = false;
        settle(&root, book, &log.latest, open, interrupted).await;
        if !*active || !open.is_empty() || working() {
            return false;
        }
        *active = false;
        write(&root, book, &log.latest, B::inactive()).await;
        true
    })
    .await
    .unwrap_or(false)
}

/// Rule 3: the ask kept for the run of `epoch`, open: its index, and
/// whether it made the run active; `None` when it was not kept.
pub async fn begin<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, epoch: u64, ask: <B::Entry as Entry>::Item, kind: Ask) -> Option<(u64, bool)> {
    whole(async move {
        let mut state = log.lock.lock().await;
        let Lifecycle::Deployed { epoch: deployed, active, open, .. } = &mut *state else {
            return None;
        };
        if *deployed != epoch {
            return None;
        }
        let began = open.is_empty() && !*active;
        if began {
            *active = true;
            write(&root, book, &log.latest, B::active()).await;
        }
        let index = write(&root, book, &log.latest, ask).await?;
        open.insert(index, kind);
        Some((index, began))
    })
    .await
    .flatten()
}

/// Rules 3 and 3a: the ask at `index` answered for the run of `epoch`
/// — or, interrupted, marked so and left open. An answer kept proves
/// the run alive: every ask marked interrupted is closed first.
/// Whether its answer left the run inactive.
pub async fn end<B: RunBook>(
    root: PathBuf,
    book: B,
    log: Arc<RunLog>,
    epoch: u64,
    index: u64,
    answer: Answer<<B::Entry as Entry>::Item>,
) -> bool {
    whole(async move {
        let mut state = log.lock.lock().await;
        let Lifecycle::Deployed { epoch: deployed, active, open, interrupted, .. } = &mut *state else {
            return false;
        };
        if *deployed != epoch || !open.contains_key(&index) {
            return false;
        }
        let Answer::Kept(answer) = answer else {
            interrupted.insert(index);
            return false;
        };
        open.remove(&index);
        interrupted.remove(&index);
        settle(&root, book, &log.latest, open, interrupted).await;
        write(&root, book, &log.latest, answer).await;
        if open.is_empty() && *active {
            *active = false;
            write(&root, book, &log.latest, B::inactive()).await;
            return true;
        }
        false
    })
    .await
    .unwrap_or(false)
}

/// Rule 4: the run of `epoch` down, for `reason`. Whether it was
/// active.
pub async fn undeployed<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, epoch: u64, reason: Option<Reason>) -> bool {
    whole(async move {
        let mut state = log.lock.lock().await;
        let Lifecycle::Deployed { epoch: deployed, .. } = &*state else {
            return false;
        };
        if *deployed != epoch {
            return false;
        }
        close(&root, book, &log, &mut state, reason).await
    })
    .await
    .unwrap_or(false)
}

/// Any other item — a chunk, a notification, an error, a command —
/// kept under the lock, moving nothing; not on a forgotten log. Its
/// index, or `None`.
pub async fn note<B: RunBook>(root: PathBuf, book: B, log: Arc<RunLog>, item: <B::Entry as Entry>::Item) -> Option<u64> {
    whole(async move {
        let state = log.lock.lock().await;
        if let Lifecycle::Forgotten = &*state {
            return None;
        }
        write(&root, book, &log.latest, item).await
    })
    .await
    .flatten()
}

/// Rule 5: nothing is kept on the log again.
pub async fn forget(log: Arc<RunLog>) {
    whole(async move {
        *log.lock.lock().await = Lifecycle::Forgotten;
    })
    .await;
}
