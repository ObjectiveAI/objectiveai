//! TAB-TO-TAB messaging: one mailbox per `(parent tab, child key)`.
//!
//! Tabs are isolated by construction — each is its own webview, its own
//! document — and the only thing a parent could hand a child was
//! `TabKind.arguments`, delivered once at boot and never again. This is
//! how two tabs keep talking.
//!
//! A tab NAMES each tab it spawns (`OpenTab.key`). From then on the
//! parent addresses that child by key and the child addresses its
//! parent with no key at all: the shell knows who spawned it, so a
//! child cannot name (or misname) anyone.
//!
//! Each mailbox holds two independent LANES — parent→child and
//! child→parent — and each lane has ONE legitimate reader, so its
//! cursor IS that reader's. The guarantees:
//!
//! - [`Lane::drain`] never yields the same item twice, so a subscriber
//!   that loops sees each message exactly once.
//! - [`TabMail::send`] queues unconditionally. A parent may send the
//!   instant it spawns, before the child has booted; the child drains
//!   it on its first subscribe.
//! - A blocked subscribe wakes when its peer CLOSES, not only when a
//!   message lands — otherwise a child waiting on a parent that just
//!   went away would park forever. Once a peer is closed, subscribing
//!   no longer blocks at all.
//! - The mailbox lives until BOTH tabs are closed. While either side
//!   lives it still accepts sends, subscribes and lists.
//!
//! This module holds the crate's only PARKED commands (a subscribe
//! with nothing pending awaits a [`tokio::sync::Notify`]). The shape
//! is [`crate::daemon_proxy`]'s waiter registry: a keyed map, a
//! `select!` against the wake source, and cleanup on every exit path.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Retained items per lane. Past this the OLDEST are dropped — a
/// chatty pair cannot grow the viewer's memory without bound, and a
/// subscriber that keeps up never notices.
const LANE_CAPACITY: usize = 1024;

/// One direction's history plus its reader's cursor.
struct Lane {
    /// Newest last. Capped at [`LANE_CAPACITY`].
    history: VecDeque<serde_json::Value>,
    /// How many items at the BACK of `history` this lane's reader has
    /// not seen. Counting from the back (rather than holding an index)
    /// is what makes the capacity drop safe: evicting from the front
    /// cannot renumber anything still unread.
    unread: usize,
}

impl Lane {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            unread: 0,
        }
    }

    fn push(&mut self, value: serde_json::Value) {
        self.history.push_back(value);
        self.unread += 1;
        while self.history.len() > LANE_CAPACITY {
            self.history.pop_front();
        }
        // An unread item that aged out is simply gone: clamp so the
        // cursor can never claim more than exists.
        self.unread = self.unread.min(self.history.len());
    }

    /// Everything this reader hasn't seen, oldest first — and it is now
    /// seen. Never yields an item twice.
    fn drain(&mut self) -> Vec<serde_json::Value> {
        let start = self.history.len() - self.unread;
        self.unread = 0;
        self.history.iter().skip(start).cloned().collect()
    }

    /// The full retained history, cursor untouched.
    fn all(&self) -> Vec<serde_json::Value> {
        self.history.iter().cloned().collect()
    }
}

/// Which end of a mailbox a caller is speaking as. A lane is written
/// by one end and read by the other, so this picks both.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Parent,
    Child,
}

struct Mailbox {
    parent: u64,
    /// Bound when the child tab is opened. `None` never happens in
    /// practice (binding is what creates the mailbox) but keeps the
    /// type honest about the reverse index.
    child: Option<u64>,
    parent_alive: bool,
    child_alive: bool,
    /// Written by the parent, read by the child.
    to_child: Lane,
    /// Written by the child, read by the parent.
    to_parent: Lane,
    /// Notified on every send AND on either side's close — the two
    /// things that can end a wait.
    wake: Arc<tokio::sync::Notify>,
}

impl Mailbox {
    /// The lane a side READS, and whether that lane's writer is still
    /// alive (a dead writer means never block).
    fn read_lane(&mut self, side: Side) -> (&mut Lane, bool) {
        match side {
            Side::Parent => (&mut self.to_parent, self.child_alive),
            Side::Child => (&mut self.to_child, self.parent_alive),
        }
    }

    fn write_lane(&mut self, side: Side) -> &mut Lane {
        match side {
            Side::Parent => &mut self.to_child,
            Side::Child => &mut self.to_parent,
        }
    }

    fn dead(&self) -> bool {
        !self.parent_alive && !self.child_alive
    }
}

#[derive(Default)]
struct MailInner {
    boxes: HashMap<(u64, String), Mailbox>,
    /// Child tab id → its mailbox key, so the child-side commands need
    /// no key of their own.
    by_child: HashMap<u64, (u64, String)>,
}

/// The viewer's tab mailboxes. Managed state, deliberately NOT part of
/// [`crate::shell::ShellModel`]: the model is pure state whose methods
/// hold their lock for the whole body and must never park, and a
/// subscribe parks by design.
#[derive(Default)]
pub struct TabMail {
    inner: tokio::sync::Mutex<MailInner>,
}

impl TabMail {
    /// Bind `child` as `parent`'s tab under `key`, creating the mailbox
    /// if it is new.
    ///
    /// IDEMPOTENT by necessity: `tabs_open` is open-or-FOCUS, so
    /// re-opening a child that already exists returns the same tab id
    /// without re-booting it. That must rejoin the existing mailbox,
    /// never reset its history or cursors. Tab ids are monotonic and
    /// never reused, so a rebind can only ever be the same child or a
    /// genuinely new one replacing a closed predecessor.
    pub async fn bind(&self, parent: u64, key: String, child: u64) {
        let mut inner = self.inner.lock().await;
        let entry = inner
            .boxes
            .entry((parent, key.clone()))
            .or_insert_with(|| Mailbox {
                parent,
                child: None,
                parent_alive: true,
                child_alive: true,
                to_child: Lane::new(),
                to_parent: Lane::new(),
                wake: Arc::new(tokio::sync::Notify::new()),
            });
        if entry.child != Some(child) {
            entry.child = Some(child);
            // A new tab under a key whose predecessor closed: the
            // history stays (it was addressed to this key, not to that
            // tab), but the slot is live again.
            entry.child_alive = true;
        }
        inner.by_child.insert(child, (parent, key));
    }

    /// Resolve the mailbox a CHILD tab belongs to.
    async fn child_key(&self, child: u64) -> Option<(u64, String)> {
        self.inner.lock().await.by_child.get(&child).cloned()
    }

    /// Queue one value on the child→parent lane ON BEHALF of a child
    /// tab that cannot invoke — the CEF script bridge's entry point
    /// (`cef::runtime`'s `ViewerClient`): a browser tab has no
    /// webview, so `tabs_parent_send` can never fire for it, but its
    /// injected script's messages should land exactly where a child
    /// tab's would. Errors when the browser was opened without a
    /// `key` (no mailbox to land in) — the caller logs-and-drops.
    pub async fn send_from_child_tab(
        &self,
        child: u64,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        let Some(id) = inner.by_child.get(&child).cloned() else {
            return Err(
                "no mailbox — the browser tab was opened without a `key`".to_string()
            );
        };
        let mailbox = inner
            .boxes
            .get_mut(&id)
            .ok_or_else(|| format!("no tab mailbox for key {:?}", id.1))?;
        mailbox.write_lane(Side::Child).push(value);
        mailbox.wake.notify_waiters();
        Ok(())
    }

    /// [`Self::subscribe`] as the child `child` — the bridge twin of
    /// `tabs_parent_subscribe`, for a browser tab's injected script
    /// (which has no webview to invoke from).
    pub async fn subscribe_from_child_tab(
        &self,
        child: u64,
        timeout: Option<std::time::Duration>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let id = self.child_key(child).await.ok_or_else(|| {
            "no mailbox — the browser tab was opened without a `key`".to_string()
        })?;
        self.subscribe(&id, Side::Child, timeout).await
    }

    /// [`Self::list`] as the child `child` — the bridge twin of
    /// `tabs_parent_list`.
    pub async fn list_from_child_tab(
        &self,
        child: u64,
        pending: bool,
    ) -> Result<Vec<serde_json::Value>, String> {
        let id = self.child_key(child).await.ok_or_else(|| {
            "no mailbox — the browser tab was opened without a `key`".to_string()
        })?;
        self.list(&id, Side::Child, pending).await
    }

    /// Which tab is bound at `id`, if one is still live there.
    ///
    /// The mailbox outlives its tabs, so this answers `None` for a
    /// child that has already closed even though the history remains.
    pub async fn child_tab(&self, id: &(u64, String)) -> Option<u64> {
        let inner = self.inner.lock().await;
        let mailbox = inner.boxes.get(id)?;
        mailbox.child_alive.then_some(mailbox.child).flatten()
    }

    /// Queue one value on the lane `side` writes. Always succeeds while
    /// the mailbox exists — a parent may send before its child has
    /// booted, and the child drains it on its first subscribe.
    pub async fn send(
        &self,
        id: &(u64, String),
        side: Side,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        let mailbox = inner
            .boxes
            .get_mut(id)
            .ok_or_else(|| format!("no tab mailbox for key {:?}", id.1))?;
        mailbox.write_lane(side).push(value);
        mailbox.wake.notify_waiters();
        Ok(())
    }

    /// The retained history of the lane `side` reads. `pending` drains
    /// (advancing the cursor, exactly like a non-blocking subscribe);
    /// otherwise the full history comes back and the cursor is
    /// untouched — the `--pending` / `--all` split the channel and
    /// agent log commands take.
    pub async fn list(
        &self,
        id: &(u64, String),
        side: Side,
        pending: bool,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut inner = self.inner.lock().await;
        let mailbox = inner
            .boxes
            .get_mut(id)
            .ok_or_else(|| format!("no tab mailbox for key {:?}", id.1))?;
        let (lane, _) = mailbox.read_lane(side);
        Ok(if pending { lane.drain() } else { lane.all() })
    }

    /// Everything pending for `side`, waiting for it if there is
    /// nothing yet.
    ///
    /// Returns IMMEDIATELY when anything is pending, and never yields
    /// an item twice. Blocks only on an empty lane — until a message
    /// arrives, until the peer CLOSES (a wait must not outlive the tab
    /// it is waiting on), or until `timeout` elapses. `None` waits
    /// forever. A peer that is already closed never blocks at all.
    pub async fn subscribe(
        &self,
        id: &(u64, String),
        side: Side,
        timeout: Option<std::time::Duration>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
        loop {
            // Subscribe to the wake BEFORE re-checking the lane: a
            // send landing between the check and the await would
            // otherwise be missed for a whole timeout.
            let wake = {
                let mut inner = self.inner.lock().await;
                let mailbox = inner
                    .boxes
                    .get_mut(id)
                    .ok_or_else(|| format!("no tab mailbox for key {:?}", id.1))?;
                let wake = Arc::clone(&mailbox.wake);
                let (lane, writer_alive) = mailbox.read_lane(side);
                let pending = lane.drain();
                if !pending.is_empty() || !writer_alive {
                    return Ok(pending);
                }
                wake
            };
            let notified = wake.notified();
            match deadline {
                Some(deadline) => {
                    if tokio::time::timeout_at(deadline, notified).await.is_err() {
                        return Ok(Vec::new());
                    }
                }
                None => notified.await,
            }
        }
    }

    /// A tab closed. Marks it dead in whichever role(s) it holds, wakes
    /// everything waiting on it, and drops any mailbox whose two ends
    /// are now both gone.
    ///
    /// Called from EVERY removal path — a tab that dies without this
    /// leaves its peer parked forever.
    pub async fn closed(&self, tab: u64) {
        let mut inner = self.inner.lock().await;
        inner.by_child.remove(&tab);
        inner.boxes.retain(|_, mailbox| {
            let mut touched = false;
            if mailbox.parent == tab && mailbox.parent_alive {
                mailbox.parent_alive = false;
                touched = true;
            }
            if mailbox.child == Some(tab) && mailbox.child_alive {
                mailbox.child_alive = false;
                touched = true;
            }
            if touched {
                // Wake the surviving side so its wait ends now rather
                // than hanging on a peer that no longer exists.
                mailbox.wake.notify_waiters();
            }
            !mailbox.dead()
        });
    }

    /// [`Self::closed`] for a whole window's worth of tabs.
    pub async fn closed_many(&self, tabs: &[u64]) {
        for tab in tabs {
            self.closed(*tab).await;
        }
    }
}

/// Resolve the calling webview to its mailbox address as a PARENT:
/// the caller's own tab id plus the key it named the child with.
async fn as_parent(
    webview: &tauri::Webview,
    key: String,
) -> Result<(u64, String), String> {
    let parent = super::native::tab_id(webview.label())
        .ok_or_else(|| "tab messaging: not a content webview".to_string())?;
    Ok((parent, key))
}

/// Resolve the calling webview to its mailbox address as a CHILD. The
/// key is the one its parent named it with — a child never supplies
/// (or forges) an address.
async fn as_child(
    webview: &tauri::Webview,
    mail: &TabMail,
) -> Result<(u64, String), String> {
    let child = super::native::tab_id(webview.label())
        .ok_or_else(|| "tab messaging: not a content webview".to_string())?;
    mail.child_key(child)
        .await
        .ok_or_else(|| "tab messaging: this tab was not spawned with a key".to_string())
}

fn duration(timeout_ms: Option<u64>) -> Option<std::time::Duration> {
    timeout_ms.map(std::time::Duration::from_millis)
}

// ── Parent-side commands: address a spawned tab by its key ──────────

#[tauri::command]
pub async fn tabs_send(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let id = as_parent(&webview, key).await?;
    mail.send(&id, Side::Parent, value).await
}

#[tauri::command]
pub async fn tabs_subscribe(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    key: String,
    timeout_ms: Option<u64>,
) -> Result<Vec<serde_json::Value>, String> {
    let id = as_parent(&webview, key).await?;
    mail.subscribe(&id, Side::Parent, duration(timeout_ms)).await
}

#[tauri::command]
pub async fn tabs_list(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    key: String,
    pending: bool,
) -> Result<Vec<serde_json::Value>, String> {
    let id = as_parent(&webview, key).await?;
    mail.list(&id, Side::Parent, pending).await
}

/// Close the tab this one spawned as `key`.
///
/// The ONLY scoped close in the shell: `tabs_close` takes a raw tab id
/// and checks nothing, but a caller has no sanctioned way to learn a
/// child's id — and this resolves the key through the mailbox index,
/// so a tab can only ever reach a tab it spawned itself.
///
/// A child that is already gone is not an error: the mailbox outlives
/// its tabs, so "close it" is satisfied either way.
#[tauri::command]
pub async fn tabs_close_child(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    key: String,
) -> Result<(), String> {
    let id = as_parent(&webview, key).await?;
    if let Some(child) = mail.child_tab(&id).await {
        super::close_tab(&app, child).await;
    }
    Ok(())
}

// ── Child-side commands: address the spawner, no key needed ─────────

#[tauri::command]
pub async fn tabs_parent_send(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    value: serde_json::Value,
) -> Result<(), String> {
    let id = as_child(&webview, &mail).await?;
    mail.send(&id, Side::Child, value).await
}

#[tauri::command]
pub async fn tabs_parent_subscribe(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    timeout_ms: Option<u64>,
) -> Result<Vec<serde_json::Value>, String> {
    let id = as_child(&webview, &mail).await?;
    mail.subscribe(&id, Side::Child, duration(timeout_ms)).await
}

#[tauri::command]
pub async fn tabs_parent_list(
    webview: tauri::Webview,
    mail: tauri::State<'_, TabMail>,
    pending: bool,
) -> Result<Vec<serde_json::Value>, String> {
    let id = as_child(&webview, &mail).await?;
    mail.list(&id, Side::Child, pending).await
}
