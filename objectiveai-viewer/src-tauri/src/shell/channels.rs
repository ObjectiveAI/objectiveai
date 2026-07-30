//! The resident `/channels` listener: every incoming channel OFFER
//! opens a channel-request tab — into the FOCUSED window, activated
//! (auto-swap) — under the PUBLISHING PLUGIN's identity. The viewer
//! is the user surface for offers — `channels publish` blocks
//! daemon-side until some client accepts, and this is where a human
//! gets to look at one.
//!
//! Lifecycle rules — the DAEMON owns the open set, the shell MODEL is
//! the only registry (no channel state is held here):
//! - Offers REPLAY on every (re)connect; an offer whose request tab
//!   is already LIVE in the model (matched by `channel_id` in the
//!   template tab's arguments) is skipped. A locally DECLINED offer
//!   (tab closed — there is no decline wire op; ignoring IS
//!   declining) stays dismissed only until the next reconnect: still
//!   open server-side means shown again, the daemon's truth.
//! - `offer_withdrawn` → the matching live tab closes.
//! - Withdrawals during a disconnect are NEVER delivered (the daemon
//!   notifies only connections that saw the offer), so each connect
//!   RECONCILES at the `live` marker: live request tabs the replay
//!   did not name are gone server-side — close them.
//!
//! Rust hardcodes no tab modules (and cannot know dev vs prod), so
//! the chrome DECLARES the channel-request component's coordinates
//! ([`channel_request_declare`], first-wins like `tabs_declare`); the
//! listener does not connect until that declaration lands — offers
//! must never arrive unopenable.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use objectiveai_sdk::daemon::channel_listener::{ChannelEvent, ChannelOffer};
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
/// component as a new tab (same window, activated + focused) whose
/// arguments are `{ request, response }` — the published offer and
/// the accept's response body (`{ secret }`). The HANDLER owns the
/// capability outright: it sends the secret itself on its channels
/// commands; Rust neither stores nor stamps it. Then the request tab
/// closes. ANY failure kills the request tab exactly as if it were
/// closed.
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
    let identity_args = &offer.identity;
    let (owner, name, version) = match (
        &identity_args.plugin_owner,
        &identity_args.plugin_name,
        &identity_args.plugin_version,
    ) {
        (Some(owner), Some(name), Some(version)) => (owner, name, version),
        _ => return Err("offer has no publishing plugin".to_string()),
    };

    // Resolve the handler FIRST: no handler, no accept. Accept only
    // renders in the `ready` state, so the non-ready arms are
    // stale-state races — still distinct in the log.
    let state = app.state::<ChannelRequests>();
    let handler = match super::plugins::channel_status(
        app,
        &state.plugins_root,
        owner,
        name,
        version,
        &offer.key,
    )
    .await
    {
        super::plugins::ChannelStatus::Ready(handler) => handler,
        super::plugins::ChannelStatus::NotInstalled => {
            return Err(format!("{owner}/{name}/{version} is not installed"));
        }
        super::plugins::ChannelStatus::UnsupportedKey => {
            return Err(format!(
                "{owner}/{name}/{version} declares no channel handler for key {:?}",
                offer.key
            ));
        }
    };

    let proxy = app.state::<crate::daemon_proxy::DaemonProxy>();
    let secret = proxy.accept_channel(&offer.channel_id).await?;

    let identity = format!(
        "{}/{}/{}",
        owner.to_lowercase(),
        name.to_lowercase(),
        version
    );
    // Handler tabs are titled by their offer key — the manifest's
    // Channel entries deliberately carry no title.
    let title = offer.key.clone();
    // The handler's whole world: the published offer and what the
    // accept came back with. The secret rides IN — the handler owns
    // the capability and sends it itself on its channels commands.
    let arguments = serde_json::json!({
        "request": offer,
        "response": objectiveai_sdk::daemon::channel_listener::ChannelAccepted {
            secret,
        },
    });
    let kind = TabKind {
        identity,
        key: None,
        arguments: Some(arguments),
        surface: super::Surface::Component {
            module: handler.module,
            export: handler.export,
            root_module: false,
        },
    };
    let opened = super::open_tab(
        app,
        window,
        kind,
        title,
        true,
        handler.icon,
        handler.styles,
        true,
    )
    .await;
    // The request tab dies; the handler tab takes its place, focused.
    super::close_tab(app, request_tab).await;
    super::select_tab(app, window, opened.tab_id).await;
    Ok(())
}

/// One offer's standing against the installed-plugin tree — what the
/// request tab renders its verbs from.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferStatus {
    /// Installed and the key maps to a handler — Accept works.
    Ready,
    /// The publishing plugin's exact version is not installed.
    NotInstalled,
    /// Installed, but the key is absent from the manifest's channel
    /// tabs (or the plugin has no viewer extension).
    UnsupportedKey,
    /// The offer carries no publishing plugin — nothing to install,
    /// nothing to accept.
    NoPlugin,
}

/// The CALLING request tab's offer status — self-scoped like accept:
/// the offer comes from the caller's own tab arguments.
#[tauri::command]
pub async fn channel_request_status(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<OfferStatus, String> {
    let request_tab = native::tab_id(webview.label())
        .ok_or_else(|| "channel status: not a content webview".to_string())?;
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
    let identity_args = &offer.identity;
    let (owner, name, version) = match (
        &identity_args.plugin_owner,
        &identity_args.plugin_name,
        &identity_args.plugin_version,
    ) {
        (Some(owner), Some(name), Some(version)) => (owner, name, version),
        _ => return Ok(OfferStatus::NoPlugin),
    };
    let state = app.state::<ChannelRequests>();
    Ok(
        match super::plugins::channel_status(
            &app,
            &state.plugins_root,
            owner,
            name,
            version,
            &offer.key,
        )
        .await
        {
            super::plugins::ChannelStatus::Ready(_) => OfferStatus::Ready,
            super::plugins::ChannelStatus::NotInstalled => OfferStatus::NotInstalled,
            super::plugins::ChannelStatus::UnsupportedKey => {
                OfferStatus::UnsupportedKey
            }
        },
    )
}

/// Install the CALLING request tab's publishing plugin — the offer
/// tab's Install button. Runs the daemon-build download pipeline with
/// the two-step progress feed, then rescans the inventory (the
/// plugin's boot tabs appear quietly). ANY failure surfaces in
/// viewer-logs and kills the request tab, exactly like a failed
/// accept. Success leaves the tab alive — the caller re-queries
/// [`channel_request_status`] to decide what it shows next.
#[tauri::command]
pub async fn channel_request_install(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    on_step: tauri::ipc::Channel<super::InstallStep>,
) -> Result<(), String> {
    let request_tab = native::tab_id(webview.label())
        .ok_or_else(|| "channel install: not a content webview".to_string())?;
    let window = webview.window().label().to_string();
    let result = install_flow(&app, request_tab, &window, on_step).await;
    if let Err(message) = &result {
        super::report_shell(&app, "error", format!("channels: install: {message}"))
            .await;
        super::close_tab(&app, request_tab).await;
    }
    result
}

/// The fallible body of [`channel_request_install`] — see its doc.
async fn install_flow(
    app: &tauri::AppHandle,
    request_tab: u64,
    window: &str,
    on_step: tauri::ipc::Channel<super::InstallStep>,
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
    let identity_args = &offer.identity;
    let (owner, name, version) = match (
        &identity_args.plugin_owner,
        &identity_args.plugin_name,
        &identity_args.plugin_version,
    ) {
        (Some(owner), Some(name), Some(version)) => (owner, name, version),
        _ => return Err("offer has no publishing plugin".to_string()),
    };
    let state = app.state::<ChannelRequests>();
    // Probe first: an already-installed plugin (raced by another
    // install) is SUCCESS here — the status re-query decides what the
    // tab shows.
    if !matches!(
        super::plugins::channel_status(
            app,
            &state.plugins_root,
            owner,
            name,
            version,
            &offer.key,
        )
        .await,
        super::plugins::ChannelStatus::NotInstalled
    ) {
        return Ok(());
    }
    let proxy = app.state::<crate::daemon_proxy::DaemonProxy>();
    let dirs = app.state::<super::PluginsDirs>();
    super::install(
        app,
        proxy.daemon(),
        &dirs,
        owner,
        name,
        version,
        Some(&on_step),
    )
    .await?;
    super::report_shell(
        app,
        "info",
        format!("channels: installed {owner}/{name}@{version} from the offer tab"),
    )
    .await;
    super::rescan_and_apply(app, &state.plugins_root, window, true).await;
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
                let shown = request_tabs(app)
                    .await
                    .iter()
                    .any(|(channel_id, _)| channel_id == &offer.channel_id);
                if !shown {
                    handle_offer(app, plugins_root, offer).await;
                }
            }
            ChannelEvent::OfferWithdrawn { channel_id } => {
                let tabs: Vec<u64> = request_tabs(app)
                    .await
                    .into_iter()
                    .filter(|(id, _)| id == &channel_id)
                    .map(|(_, tab_id)| tab_id)
                    .collect();
                for tab_id in tabs {
                    super::close_tab(app, tab_id).await;
                }
            }
            ChannelEvent::Live => {
                live = true;
                // Reconcile: live request tabs the replay did not name
                // vanished server-side while we were disconnected.
                let stale: Vec<u64> = request_tabs(app)
                    .await
                    .into_iter()
                    .filter(|(id, _)| !replayed.contains(id))
                    .map(|(_, tab_id)| tab_id)
                    .collect();
                for tab_id in stale {
                    super::close_tab(app, tab_id).await;
                }
            }
        }
    }
}

/// Every LIVE channel-request tab as `(channel_id, tab_id)`, derived
/// from the model — the tabs ARE the shown-offer registry. A request
/// tab is the declared template component; its arguments are the
/// wire offer, whose top-level `channel_id` identifies it (handler
/// tabs nest their offer under `request`, so they never match).
async fn request_tabs(app: &tauri::AppHandle) -> Vec<(String, u64)> {
    let template = {
        let state = app.state::<ChannelRequests>();
        let inner = state.inner.lock().await;
        match &inner.template {
            Some(t) => (t.module.clone(), t.export.clone()),
            None => return Vec::new(),
        }
    };
    let model = app.state::<ShellModel>();
    model
        .windows_full()
        .await
        .into_values()
        .flat_map(|window| window.tabs)
        .filter(|tab| {
            matches!(
                &tab.kind.surface,
                super::Surface::Component { module, export, .. }
                    if module == &template.0 && export == &template.1
            )
        })
        .filter_map(|tab| {
            let channel_id = tab
                .kind
                .arguments
                .as_ref()?
                .get("channel_id")?
                .as_str()?
                .to_string();
            Some((channel_id, tab.id))
        })
        .collect()
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
    let identity_args = &offer.identity;
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
        // Nobody SPAWNED this tab by key — the listener opens it.
        key: None,
        // The whole wire offer, verbatim — the tab is a pure render
        // of it, and the embedded channel_id keeps every kind unique.
        arguments: Some(arguments),
        surface: super::Surface::Component {
            module: template.0,
            export: template.1,
            // The request TEMPLATE is root code even when the
            // identity is the offering plugin's — the module must not
            // be prefixed onto the plugin origin.
            root_module: true,
        },
    };
    let model = app.state::<ShellModel>();
    let Some(window) = target_window(app, &model).await else {
        return;
    };
    // Activated: an incoming request swaps to its tab (the window's
    // OS focus is left alone). No bookkeeping: the tab itself IS the
    // shown-offer record.
    super::open_tab(app, &window, kind, title, true, icon, Vec::new(), true).await;
}
