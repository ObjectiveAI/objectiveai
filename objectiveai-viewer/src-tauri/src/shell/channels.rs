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

/// One accepted channel, owned by a handler tab: the capability the
/// proxy stamps into that tab's channels command bodies.
struct TabChannel {
    channel_id: String,
    secret: String,
}

struct ChannelsInner {
    template: Option<Template>,
    /// `channel_id` → tab id. Insert on spawn; remove ONLY on
    /// withdraw/reconcile/accept — see the module doc's dedup rules.
    offers: HashMap<String, u64>,
    /// `tab id` → its accepted channel. Entries are never pruned (tab
    /// ids are never reused, so a stale entry can never stamp the
    /// wrong caller).
    secrets: HashMap<u64, TabChannel>,
}

/// Managed state for the channel-request surface.
pub struct ChannelRequests {
    inner: tokio::sync::Mutex<ChannelsInner>,
    /// The installed-plugin tree — the accept command's manifest
    /// lookups read it.
    plugins_root: PathBuf,
    /// Taken by the FIRST chrome declaration (later ones no-op); the
    /// paired receiver gates the listener's first connect.
    declared_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    declared_rx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

impl ChannelRequests {
    pub fn new(plugins_root: PathBuf) -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self {
            inner: tokio::sync::Mutex::new(ChannelsInner {
                template: None,
                offers: HashMap::new(),
                secrets: HashMap::new(),
            }),
            plugins_root,
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

/// Accept the CALLING channel-request tab's offer. Self-scoped like
/// `tab_self`: the offer comes from the caller's own tab arguments.
///
/// Flow: resolve the publishing plugin's manifest handler for the
/// offer key (BEFORE the POST — a secret must never be minted with
/// nowhere to land) → `POST /channels/{id}/accept` → open the handler
/// component as a new tab (same window, activated + focused) with the
/// FULL offer as its arguments → record the tab's secret for the
/// proxy to stamp → close the request tab. ANY failure kills the
/// request tab exactly as if it were closed.
#[tauri::command]
pub async fn channel_request_accept(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<(), String> {
    let request_tab = native::tab_id(webview.label())
        .ok_or_else(|| "channel accept: not a content webview".to_string())?;
    let window = webview.window().label().to_string();

    // Kill-the-tab-on-failure wrapper.
    let result = accept_flow(&app, request_tab, &window).await;
    if let Err(message) = &result {
        super::report_shell(&app, "error", format!("channels: accept: {message}")).await;
        super::close_tab(&app, request_tab).await;
    }
    result
}

/// The fallible body of [`channel_request_accept`] — see its doc.
async fn accept_flow(
    app: &tauri::AppHandle,
    request_tab: u64,
    window: &str,
) -> Result<(), String> {
    let model = app.state::<ShellModel>();
    let tab = model
        .tab(request_tab)
        .await
        .ok_or_else(|| "unknown tab".to_string())?;
    let arguments = tab
        .kind
        .arguments
        .ok_or_else(|| "tab carries no offer".to_string())?;
    let offer: ChannelOffer = serde_json::from_value(arguments)
        .map_err(|e| format!("offer parse: {e}"))?;
    let identity_args = &offer.agent_arguments;
    let (owner, name, version) = match (
        &identity_args.plugin_owner,
        &identity_args.plugin_name,
        &identity_args.plugin_version,
    ) {
        (Some(owner), Some(name), Some(version)) => (owner, name, version),
        _ => return Err("offer has no publishing plugin".to_string()),
    };

    // Resolve the handler FIRST: no handler, no accept.
    let state = app.state::<ChannelRequests>();
    let handler = super::plugins::plugin_channel(
        &state.plugins_root,
        owner,
        name,
        version,
        &offer.key,
    )
    .await
    .ok_or_else(|| {
        format!(
            "{owner}/{name}/{version} declares no channel handler for key {:?}",
            offer.key
        )
    })?;

    let proxy = app.state::<crate::daemon_proxy::DaemonProxy>();
    let secret = proxy.accept_channel(&offer.channel_id).await?;

    // Accepted: the offer is gone server-side — drop our mapping so
    // the daemon's own offer_withdrawn broadcast finds nothing.
    {
        let mut inner = state.inner.lock().await;
        inner.offers.remove(&offer.channel_id);
    }

    let identity = format!(
        "{}/{}/{}",
        owner.to_lowercase(),
        name.to_lowercase(),
        version
    );
    let title = handler
        .title
        .clone()
        .unwrap_or_else(|| offer.key.clone());
    let arguments = serde_json::to_value(&offer)
        .map_err(|e| format!("offer serialize: {e}"))?;
    let kind = TabKind {
        identity,
        module: handler.module,
        export: handler.export,
        arguments: Some(arguments),
    };
    let opened =
        super::open_tab(app, window, kind, title, true, handler.icon, true).await;
    {
        let mut inner = state.inner.lock().await;
        inner.secrets.insert(
            opened.tab_id,
            TabChannel {
                channel_id: offer.channel_id.clone(),
                secret,
            },
        );
    }
    // The request tab dies; the handler tab takes its place, focused.
    super::close_tab(app, request_tab).await;
    super::select_tab(app, window, opened.tab_id).await;
    Ok(())
}

/// Stamp the calling tab's channel secret into a channels command
/// body. A no-op unless the webview is a content webview whose tab
/// owns a channel AND `request` is a `channels/*` command targeting
/// that exact channel — then its `secret` field is overwritten (the
/// component sends a placeholder; the capability stays Rust-side).
pub(crate) async fn stamp_channel_secret(
    requests: &ChannelRequests,
    webview_label: &str,
    request: String,
) -> String {
    let Some(tab_id) = native::tab_id(webview_label) else {
        return request;
    };
    let entry = {
        let inner = requests.inner.lock().await;
        match inner.secrets.get(&tab_id) {
            Some(entry) => TabChannel {
                channel_id: entry.channel_id.clone(),
                secret: entry.secret.clone(),
            },
            None => return request,
        }
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&request) else {
        return request;
    };
    let Some(object) = value.as_object_mut() else {
        return request;
    };
    let is_channels = object
        .get("path_type")
        .and_then(|v| v.as_str())
        .is_some_and(|path| path.starts_with("channels/"));
    let matches_channel = object
        .get("channel_id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id == entry.channel_id);
    if !is_channels || !matches_channel {
        return request;
    }
    object.insert(
        "secret".to_string(),
        serde_json::Value::String(entry.secret),
    );
    serde_json::to_string(&value).unwrap_or(request)
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
    let identity_args = &offer.agent_arguments;
    let trio = match (
        &identity_args.plugin_owner,
        &identity_args.plugin_name,
        &identity_args.plugin_version,
    ) {
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
