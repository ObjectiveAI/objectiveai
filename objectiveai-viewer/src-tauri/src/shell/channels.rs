//! The resident `/channels` listener: every incoming channel OFFER
//! opens a channel-request tab — into the FOCUSED window, activated
//! (auto-swap) — under the PUBLISHING PLUGIN's identity. The viewer
//! is the user surface for offers — `channels publish` blocks
//! daemon-side until some client accepts, and this is where a human
//! gets to look at one.
//!
//! Lifecycle rules:
//! - Offers REPLAY on every (re)connect, so the listener dedups by
//!   `channel_id` → tab id. A map entry outlives its tab: a locally
//!   DECLINED offer (tab closed, still open server-side — there is no
//!   decline wire op; ignoring IS declining) stays dismissed for this
//!   viewer's life.
//! - `offer_withdrawn` → the mapped tab closes.
//! - Withdrawals during a disconnect are NEVER delivered (the daemon
//!   notifies only connections that saw the offer), so each connect
//!   RECONCILES at the `live` marker: mapped offers the replay did
//!   not include are gone server-side — close them.
//!
//! Rust hardcodes no tab modules (and cannot know dev vs prod), so
//! the chrome DECLARES the channel-request component's coordinates
//! ([`channel_request_declare`], first-wins like `tabs_declare`); the
//! listener does not connect until that declaration lands — offers
//! must never arrive unopenable.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use futures::StreamExt;
use objectiveai_sdk::cli::channel_listener::{ChannelEvent, ChannelOffer};
use tauri::Manager;

use super::model::{ROOT_IDENTITY, ShellModel, TabKind};
use super::native;

/// The chrome-declared channel-request component coordinates.
struct Template {
    module: String,
    export: Option<String>,
}

struct ChannelsInner {
    template: Option<Template>,
    /// `channel_id` → tab id. Insert on spawn; remove ONLY on
    /// withdraw/reconcile — see the module doc's dedup rules.
    offers: HashMap<String, u64>,
}

/// Managed state for the channel-request surface.
pub struct ChannelRequests {
    inner: tokio::sync::Mutex<ChannelsInner>,
    /// Taken by the FIRST chrome declaration (later ones no-op); the
    /// paired receiver gates the listener's first connect.
    declared_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    declared_rx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

impl Default for ChannelRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelRequests {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self {
            inner: tokio::sync::Mutex::new(ChannelsInner {
                template: None,
                offers: HashMap::new(),
            }),
            declared_tx: tokio::sync::Mutex::new(Some(tx)),
            declared_rx: tokio::sync::Mutex::new(Some(rx)),
        }
    }
}

/// The chrome declares the channel-request tab's component — how Rust
/// learns the module without hardcoding a path. Every chrome calls
/// this on mount; only the FIRST declaration per app run applies (all
/// chromes ship the same build). Chrome-only, like `tabs_declare`.
#[tauri::command]
pub async fn channel_request_declare(
    webview: tauri::Webview,
    state: tauri::State<'_, ChannelRequests>,
    module: String,
    export: Option<String>,
) -> Result<(), String> {
    let label = webview.label();
    if native::tab_id(label).is_some() || !label.starts_with("chrome-") {
        return Err("channel_request_declare: chrome webviews only".to_string());
    }
    super::validate_module(&module)?;
    {
        let mut inner = state.inner.lock().await;
        if inner.template.is_none() {
            inner.template = Some(Template { module, export });
        }
    }
    if let Some(tx) = state.declared_tx.lock().await.take() {
        let _ = tx.send(());
    }
    Ok(())
}

/// Spawn the resident listener: await the chrome's template
/// declaration, then connect/consume/reconnect forever (the
/// command-logs capture's shape — the viewer may outlive several
/// daemons).
pub fn spawn_channel_listener(app: tauri::AppHandle, plugins_root: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let rx = {
            let state = app.state::<ChannelRequests>();
            let rx = state.declared_rx.lock().await.take();
            rx
        };
        match rx {
            Some(rx) => {
                let _ = rx.await;
            }
            // A second spawn (impossible today) — just don't listen.
            None => return,
        }
        loop {
            listen_once(&app, &plugins_root).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

async fn listen_once(app: &tauri::AppHandle, plugins_root: &Path) {
    let request = app
        .state::<crate::daemon_proxy::DaemonProxy>()
        .channels_builder();
    let Ok(response) = request.send().await else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    use eventsource_stream::Eventsource;
    // Everything the pre-`live` replay named — the reconcile set.
    let mut replayed: HashSet<String> = HashSet::new();
    let mut live = false;
    let mut events = response.bytes_stream().eventsource();
    while let Some(Ok(event)) = events.next().await {
        let Ok(frame) = serde_json::from_str::<ChannelEvent>(&event.data) else {
            continue;
        };
        match frame {
            ChannelEvent::Offer { offer } => {
                if !live {
                    replayed.insert(offer.channel_id.clone());
                }
                let known = {
                    let state = app.state::<ChannelRequests>();
                    let inner = state.inner.lock().await;
                    inner.offers.contains_key(&offer.channel_id)
                };
                if !known {
                    handle_offer(app, plugins_root, offer).await;
                }
            }
            ChannelEvent::OfferWithdrawn { channel_id } => {
                let tab_id = {
                    let state = app.state::<ChannelRequests>();
                    let mut inner = state.inner.lock().await;
                    inner.offers.remove(&channel_id)
                };
                if let Some(tab_id) = tab_id {
                    super::close_tab(app, tab_id).await;
                }
            }
            ChannelEvent::Live => {
                live = true;
                // Reconcile: mapped offers the replay did not include
                // vanished while we were disconnected.
                let stale: Vec<u64> = {
                    let state = app.state::<ChannelRequests>();
                    let mut inner = state.inner.lock().await;
                    let gone: Vec<String> = inner
                        .offers
                        .keys()
                        .filter(|id| !replayed.contains(*id))
                        .cloned()
                        .collect();
                    gone.iter()
                        .filter_map(|id| inner.offers.remove(id))
                        .collect()
                };
                for tab_id in stale {
                    super::close_tab(app, tab_id).await;
                }
            }
        }
    }
}

/// The window an incoming offer's tab opens into: the FOCUSED window
/// when one of ours is, else the lowest-numbered model window (a
/// deterministic somewhere). `None` = no windows at all (the app is
/// exiting).
async fn target_window(app: &tauri::AppHandle, model: &ShellModel) -> Option<String> {
    let mut labels: Vec<String> = model.windows_full().await.into_keys().collect();
    if labels.is_empty() {
        return None;
    }
    for (label, window) in app.windows() {
        if labels.contains(&label) && window.is_focused().unwrap_or(false) {
            return Some(label);
        }
    }
    labels.sort_by_key(|label| {
        (
            label
                .strip_prefix("shell-")
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(u64::MAX),
            label.clone(),
        )
    });
    labels.into_iter().next()
}

/// One fresh offer → one regular tab, activated (auto-swap) in the
/// target window. The tab lives under the PUBLISHING plugin's
/// identity (versioned, matching the inventory's display identity)
/// with the plugin's manifest icon when one is installed; a
/// non-plugin publisher brands as the root.
async fn handle_offer(app: &tauri::AppHandle, plugins_root: &Path, offer: ChannelOffer) {
    let template = {
        let state = app.state::<ChannelRequests>();
        let inner = state.inner.lock().await;
        match &inner.template {
            Some(t) => (t.module.clone(), t.export.clone()),
            // Unreachable: the listener connects only after the
            // declaration.
            None => return,
        }
    };
    let trio = match (&offer.plugin_owner, &offer.plugin_name, &offer.plugin_version) {
        (Some(owner), Some(name), Some(version)) => {
            Some((owner.clone(), name.clone(), version.clone()))
        }
        _ => None,
    };
    let (identity, icon) = match &trio {
        Some((owner, name, version)) => (
            format!(
                "{}/{}/{}",
                owner.to_lowercase(),
                name.to_lowercase(),
                version
            ),
            super::plugins::plugin_icon(plugins_root, owner, name, version).await,
        ),
        None => (ROOT_IDENTITY.to_string(), None),
    };
    let channel_id = offer.channel_id.clone();
    let title = if offer.key.is_empty() {
        "channel request".to_string()
    } else {
        offer.key.clone()
    };
    let Ok(arguments) = serde_json::to_value(&offer) else {
        return;
    };
    let kind = TabKind {
        identity,
        module: template.0,
        export: template.1,
        // The whole wire offer, verbatim — the tab is a pure render
        // of it, and the embedded channel_id keeps every kind unique.
        arguments: Some(arguments),
    };
    let model = app.state::<ShellModel>();
    let Some(window) = target_window(app, &model).await else {
        return;
    };
    // Activated: an incoming request swaps to its tab (the window's
    // OS focus is left alone).
    let opened = super::open_tab(app, &window, kind, title, true, icon, true).await;
    let state = app.state::<ChannelRequests>();
    let mut inner = state.inner.lock().await;
    inner.offers.insert(channel_id, opened.tab_id);
}
