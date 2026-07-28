//! The development file watcher: what turns a save in a registered
//! plugin's directory into a hot reload.
//!
//! One recursive notify watcher per registered root (watching the
//! DIRECTORY, never individual files — editors and bundlers save by
//! atomic rename, which silently kills file-level watches). Events are
//! debounced (~150ms — a watch build writes several files per save;
//! one reload, not five), then classified per plugin against the LIVE
//! manifest and the attribution maps:
//!
//! - `objectiveai.json` changed → inventory rescan (tab/script lists
//!   are live).
//! - a changed file consumed by open tabs → the cheapest honest
//!   reload for each: styles-only → `dev://styles-changed` (link swap,
//!   no remount); exactly the entry module → `dev://module-changed`
//!   (component remount; document, transport, mailbox survive);
//!   anything else → webview reload (fresh document, reboots like an
//!   open).
//! - a changed file that is a declared SCRIPT module → CLOSE the
//!   browser tabs it was injected into. An executed IIFE cannot be
//!   hot-swapped; a closed browser is honest where a stale one lies.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use tauri::Manager;

/// What the resident watch task receives.
pub enum DevWatchMsg {
    /// The registry changed — drop every watcher and re-arm from
    /// `DevPlugins::roots()`.
    Rearm,
    /// The filesystem reported a change under some registered root.
    Changed(PathBuf),
}

/// The channel into the watch task, managed as state so the stdin
/// loop can re-arm after a registry update.
pub struct DevWatchTx(pub tokio::sync::mpsc::UnboundedSender<DevWatchMsg>);

/// Monotonic cache-bust token for the reload events. Uniqueness is all
/// that matters — the module map and link hrefs key on the URL string.
static VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_version() -> u64 {
    VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(150);

/// Spawn the resident watch task; returns nothing — the channel is
/// managed on the app.
pub fn spawn_dev_watch(app: tauri::AppHandle) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DevWatchMsg>();
    app.manage(DevWatchTx(tx.clone()));
    tauri::async_runtime::spawn(async move {
        // The live watchers, one per registered root. Dropping a
        // watcher stops it; rebuilding wholesale on every re-arm keeps
        // convergence trivial (registrations are few and re-arms
        // rare).
        // Held for Drop alone: a watcher watches until dropped, and
        // re-arming replaces the whole vec. Never read — the
        // underscore is for the lint.
        let mut _watchers: Vec<notify::RecommendedWatcher> = Vec::new();
        let mut pending: HashSet<PathBuf> = HashSet::new();
        loop {
            let msg = if pending.is_empty() {
                match rx.recv().await {
                    Some(msg) => Some(msg),
                    None => return,
                }
            } else {
                // Debounce window: keep absorbing events until the
                // stream goes quiet for DEBOUNCE, then flush.
                match tokio::time::timeout(DEBOUNCE, rx.recv()).await {
                    Ok(Some(msg)) => Some(msg),
                    Ok(None) => return,
                    Err(_) => None,
                }
            };
            match msg {
                Some(DevWatchMsg::Rearm) => {
                    _watchers = arm(&app, &tx);
                }
                Some(DevWatchMsg::Changed(path)) => {
                    pending.insert(path);
                }
                None => {
                    let batch = std::mem::take(&mut pending);
                    process(&app, batch).await;
                }
            }
        }
    });
}

/// Build one recursive watcher per registered root.
fn arm(
    app: &tauri::AppHandle,
    tx: &tokio::sync::mpsc::UnboundedSender<DevWatchMsg>,
) -> Vec<notify::RecommendedWatcher> {
    use notify::Watcher as _;
    let dev = app.state::<super::DevPlugins>();
    let mut watchers = Vec::new();
    for ((owner, name, version), root) in dev.roots() {
        let tx = tx.clone();
        let watcher = notify::recommended_watcher(
            move |event: Result<notify::Event, notify::Error>| {
                let Ok(event) = event else { return };
                for path in event.paths {
                    let _ = tx.send(DevWatchMsg::Changed(path));
                }
            },
        );
        match watcher {
            Ok(mut watcher) => {
                match watcher.watch(&root, notify::RecursiveMode::Recursive) {
                    Ok(()) => watchers.push(watcher),
                    Err(e) => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            super::report_shell(
                                &app,
                                "error",
                                format!(
                                    "dev: {owner}/{name}/{version}: watch failed: {e}"
                                ),
                            )
                            .await;
                        });
                    }
                }
            }
            Err(e) => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    super::report_shell(
                        &app,
                        "error",
                        format!("dev: {owner}/{name}/{version}: watcher: {e}"),
                    )
                    .await;
                });
            }
        }
    }
    watchers
}

/// Classify one debounced batch and fire the cheapest honest reload
/// for every affected consumer.
async fn process(app: &tauri::AppHandle, batch: HashSet<PathBuf>) {
    let dev = app.state::<super::DevPlugins>();
    let model = app.state::<super::ShellModel>();
    let mut rescan = false;

    for ((owner, name, version), root) in dev.roots() {
        let relevant: Vec<&PathBuf> =
            batch.iter().filter(|p| p.starts_with(&root)).collect();
        if relevant.is_empty() {
            continue;
        }
        if relevant.iter().any(|p| p.as_path() == root.join("objectiveai.json")) {
            rescan = true;
        }
        let Some(manifest) = super::dev::read_dev_manifest(&root).await else {
            continue;
        };
        let Some(asset_root) = super::dev::dev_asset_root(&root).await else {
            continue;
        };

        // Script files, resolved once per plugin per batch.
        let script_files: HashSet<PathBuf> = manifest
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.scripts.as_ref())
            .map(|scripts| {
                scripts
                    .iter()
                    .filter_map(|s| super::plugins::normalize(&s.module))
                    .map(|rel| {
                        let mut file = asset_root.clone();
                        for segment in rel.split('/').filter(|s| !s.is_empty()) {
                            file = file.join(segment);
                        }
                        file
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut tab_changes: HashMap<u64, HashSet<PathBuf>> = HashMap::new();
        for path in &relevant {
            if script_files.contains(path.as_path()) {
                // Scripts opt OUT of reload: close every browser this
                // file was injected into. Honest teardown — the
                // profile (cookies, logins) survives the close; the
                // next spawn injects the new script.
                for browser in dev.browsers_running_script(path) {
                    super::close_tab(app, browser).await;
                }
            }
            for tab in dev.tabs_consuming(path) {
                tab_changes.entry(tab).or_default().insert((*path).clone());
            }
        }

        let identity = format!("{owner}/{name}/{version}");
        for (tab, changed) in tab_changes {
            let Some(info) = model.tab(tab).await else {
                continue;
            };
            if info.kind.identity != identity {
                continue;
            }
            let entry = match &info.kind.surface {
                super::Surface::Component { module, .. } => {
                    super::plugins::normalize(module).map(|rel| {
                        let mut file = asset_root.clone();
                        for segment in rel.split('/').filter(|s| !s.is_empty()) {
                            file = file.join(segment);
                        }
                        file
                    })
                }
                _ => None,
            };
            let css_only = changed
                .iter()
                .all(|p| p.extension().is_some_and(|ext| ext == "css"));
            let label = super::native::tab_label(tab);
            if css_only {
                // Total state retention: the tab swaps its stylesheet
                // links in place, nothing remounts.
                let _ = tauri::Emitter::emit_to(
                    app,
                    label.as_str(),
                    "dev://styles-changed",
                    next_version(),
                );
            } else if entry.as_ref().is_some_and(|entry| {
                changed.len() == 1 && changed.contains(entry)
            }) {
                // Exactly the entry module: re-import cache-busted and
                // remount the component. The document survives, so the
                // transport and mailbox subscriptions do too.
                let _ = tauri::Emitter::emit_to(
                    app,
                    label.as_str(),
                    "dev://module-changed",
                    next_version(),
                );
            } else {
                // Chunks, mixed batches, anything unattributable to
                // the entry alone: a fresh document is the reliable
                // fallback — it reboots exactly like an open.
                if let Some(webview) = app.get_webview(&label) {
                    let _ = webview.reload();
                }
            }
        }
    }

    if rescan {
        // Tabs/titles/scripts lists are live — surface manifest edits.
        let plugins_root = app.state::<super::PluginsDirs>().plugins_root();
        if let Some(window) = model.windows_full().await.keys().next().cloned() {
            super::rescan_and_apply(app, &plugins_root, &window, true).await;
        }
    }
}
