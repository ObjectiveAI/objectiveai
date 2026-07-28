//! Browser tabs: the shell's side of a [`Surface::Browser`] tab.
//!
//! A component tab's surface is a Tauri webview the reconciler creates,
//! moves and closes through `app.get_webview`. A browser tab's is a
//! Chromium window CEF owns, which that machinery cannot see at all —
//! so this module is the parallel registry the reconciler consults, and
//! [`crate::cef`] is what it drives.
//!
//! Three things live here that CEF has no opinion about:
//!
//! - **Starting CEF at all.** Deferred to the first browser tab, which
//!   is what lets the Chromium runtime be a download rather than a
//!   shipped 200MB. The gate is retryable: a failed download must not
//!   poison every later spawn.
//! - **Profile claims.** A persistent profile is a SQLite store
//!   Chromium believes it owns exclusively, so exactly one browser may
//!   hold one — the claim is taken at spawn and released only once the
//!   browser is fully closed.
//! - **Script resolution.** A tab names a script; the name is looked up
//!   in the owning plugin's manifest and the file read off disk here.
//!   The plugin never hands over code to run, only the name of code it
//!   declared at install time.

use std::collections::HashMap;

use tauri::Manager;

use super::model::Surface;

/// How long a browser close may take before the shell proceeds without
/// it. The close is asynchronous and deliberately re-entrant (it
/// flushes cookies first — see [`crate::cef`]), so waiting is
/// mandatory; bounding the wait is what keeps a wedged renderer from
/// hanging a tab close, at the cost of that session's unflushed tail.
const CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Managed state: everything the browser tabs share.
#[derive(Default)]
pub struct Browsers {
    /// Serializes the lazy CEF start, and holds the profile claims by
    /// tab id. One mutex for both because both are touched only on the
    /// spawn and close paths, which are already serialized by the
    /// reconciler's own guard.
    inner: tokio::sync::Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    /// Live profile claims by tab id. Absent for an in-memory browser
    /// (which claims nothing) and for every component tab.
    profiles: HashMap<u64, crate::cef::Profile>,
    /// Tabs whose browser has been asked for — the reconciler creates
    /// asynchronously and CEF registers the browser some milliseconds
    /// later, so "did I already spawn this?" cannot be answered by
    /// asking CEF.
    spawned: std::collections::HashSet<u64>,
}

impl Browsers {
    /// Bring CEF up if it is not already, downloading the Chromium
    /// runtime on the way if this machine has never opened a browser
    /// tab. Safe to call on every spawn.
    ///
    /// Retryable by construction: nothing is memoized but success
    /// (`cef::is_initialized`), so a spawn that failed on a dropped
    /// download does not poison the next one.
    async fn ensure_started(&self, app: &tauri::AppHandle) -> Result<(), String> {
        if crate::cef::is_initialized() {
            return Ok(());
        }
        let bin_dir = app.state::<super::PluginsDirs>().bin_dir();
        let root = app.state::<crate::cef::ProfileRoot>().root().to_path_buf();
        let runtime = crate::cef::ensure_runtime(bin_dir).await?;

        // Where the pump picks up: the logfile is append-only across
        // viewer runs, so sample its length BEFORE anything is written
        // to it and this session starts at its own first line.
        let log_file = crate::cef::log_file(&root);
        let log_from = crate::cef::current_len(&log_file);

        // CEF's `initialize` must run on the MAIN thread, and we are on
        // a tokio worker. Hop, and carry the result back — a failure
        // here is a spawn error, not a silent dark browser.
        let (tx, rx) = tokio::sync::oneshot::channel();
        let dir = runtime.dir.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::cef::initialize(&dir, &root));
        })
        .map_err(|e| format!("cef initialize dispatch: {e}"))?;
        rx.await
            .map_err(|_| "cef initialize did not report back".to_string())??;

        // Started LAST and only on success, so the one thing that can
        // explain a browser failing to render is running before the
        // first browser exists.
        crate::cef::spawn_pump(app.clone(), log_file, log_from);
        Ok(())
    }
}

/// Read the script a browser tab named, from the owning plugin's
/// manifest.
///
/// `identity` is `owner/name/version`; the root identity declares no
/// manifest and therefore no scripts. A name that is not declared is an
/// ERROR, not a silently script-less browser — a plugin that asked for
/// an overlay and got a bare page would be very hard to debug.
async fn resolve_script(
    app: &tauri::AppHandle,
    tab: u64,
    identity: &str,
    script: &str,
) -> Result<String, String> {
    let segments: Vec<&str> = identity.split('/').collect();
    let [owner, name, version] = segments.as_slice() else {
        return Err(format!(
            "identity {identity:?} declares no scripts — only an installed plugin can"
        ));
    };
    let plugins_root = app.state::<super::PluginsDirs>().plugins_root();
    // Dev-aware: a registered trio reads its LIVE manifest.
    let manifest =
        super::plugins::manifest_for(app, &plugins_root, owner, name, version)
            .await
            .ok_or_else(|| format!("no installed manifest for {identity}"))?;
    let module = manifest
        .viewer
        .as_ref()
        .and_then(|viewer| viewer.scripts.as_ref())
        .and_then(|scripts| scripts.iter().find(|s| s.name == script))
        .map(|s| s.module.clone())
        .ok_or_else(|| format!("{identity} declares no script named {script:?}"))?;
    // Same canonicalization every other manifest path gets — the
    // leading `/` roots it at the installed viewer dir.
    let relative = super::plugins::normalize(&module)
        .ok_or_else(|| format!("invalid script module {module:?}"))?;

    let dev = app.state::<super::DevPlugins>();
    let mut file = match dev.get(owner, name, version) {
        // DEVELOPMENT: the script body comes off the author's watch
        // build. Snapshotted at spawn like production — which is
        // exactly why the watcher CLOSES this browser when the file
        // changes: an injected IIFE cannot be hot-swapped.
        Some(root) => super::dev::dev_asset_root(&root).await.ok_or_else(|| {
            format!(
                "{identity} is registered for development but its manifest                  declares no `viewer.development.output`"
            )
        })?,
        None => plugins_root
            .join(owner.to_lowercase())
            .join(name.to_lowercase())
            .join(version)
            .join(objectiveai_sdk::cli::plugins::VIEWER_DIR),
    };
    for segment in relative.split('/').filter(|s| !s.is_empty()) {
        file = file.join(segment);
    }
    let body = tokio::fs::read_to_string(&file)
        .await
        .map_err(|e| format!("read script {file:?}: {e}"))?;
    if dev.get(owner, name, version).is_some() {
        // file → browser attribution, so a change to THIS file closes
        // exactly this browser.
        dev.record_script(tab, file);
    }
    Ok(body)
}

/// Create the Chromium surface for a browser tab, once.
///
/// `parent` is the hosting window's native handle and `bounds` its
/// content rect in PHYSICAL pixels.
///
/// Idempotent per tab, and the SLOT IS CLAIMED FIRST — before the
/// runtime download, the profile claim, or anything else that can take
/// time. It has to be: CEF registers a browser asynchronously, so
/// "does this tab have one?" answers no for the whole spawn, and every
/// reconcile in that window would otherwise start another.
pub async fn spawn(
    app: &tauri::AppHandle,
    tab: u64,
    identity: &str,
    title: &str,
    surface: &Surface,
    parent: isize,
    bounds: (i32, i32, i32, i32),
) -> Result<(), String> {
    let Surface::Browser { url, script, state } = surface else {
        return Err("spawn: not a browser tab".to_string());
    };
    let browsers = app.state::<Browsers>();
    {
        let mut inner = browsers.inner.lock().await;
        if !inner.spawned.insert(tab) {
            return Ok(());
        }
    }
    // From here every failure hands the slot back, so the next
    // reconcile retries rather than leaving a permanently blank tab.
    let unclaim = || async {
        let mut inner = browsers.inner.lock().await;
        inner.spawned.remove(&tab);
    };
    if let Err(e) = browsers.ensure_started(app).await {
        unclaim().await;
        return Err(e);
    }

    // Resolve the script BEFORE claiming anything: a bad script name is
    // a plain spawn error and should not leave a claimed profile behind.
    let script = match script {
        Some(name) => match resolve_script(app, tab, identity, name).await {
            Ok(script) => Some(script),
            Err(e) => {
                unclaim().await;
                return Err(e);
            }
        },
        None => None,
    };

    // A `state` key means PERSIST — claim the profile. No key means the
    // browser is entirely in memory, which needs no directory and no
    // claim.
    let profile = match state {
        Some(state) => match app
            .state::<crate::cef::ProfileRoot>()
            .claim(identity, state)
            .await
        {
            Ok(profile) => Some(profile),
            Err(e) => {
                unclaim().await;
                return Err(e);
            }
        },
        None => None,
    };

    let result = crate::cef::create_browser(crate::cef::Spawn {
        tab,
        parent,
        bounds,
        cache: profile.as_ref().map(|p| p.dir.clone()),
        url: url.clone(),
        script,
        title: title.to_string(),
        app: app.clone(),
    });

    match result {
        Ok(()) => {
            if let Some(profile) = profile {
                let mut inner = browsers.inner.lock().await;
                inner.profiles.insert(tab, profile);
            }
            Ok(())
        }
        Err(e) => {
            // Nothing was created — give the profile straight back, or
            // the tab could never be reopened.
            if let Some(profile) = profile {
                profile.release();
            }
            unclaim().await;
            Err(e)
        }
    }
}

/// Close a tab's browser and wait for CEF to confirm it is gone, then
/// release its profile.
///
/// The order matters: releasing the claim before the browser is closed
/// would let a second browser attach to a store Chromium is still
/// flushing. A tab with no browser is a no-op, so every teardown path
/// may call this unconditionally.
///
/// Callers that are tearing down the surrounding WINDOW must await
/// this — the parent HWND cannot be destroyed while CEF is still
/// flushing under it. Callers merely closing a TAB should prefer
/// [`begin_close`], which returns at once.
pub async fn close(app: &tauri::AppHandle, tab: u64) {
    crate::cef::hide(tab);
    crate::cef::close(tab, CLOSE_TIMEOUT).await;
    let browsers = app.state::<Browsers>();
    let mut inner = browsers.inner.lock().await;
    inner.spawned.remove(&tab);
    if let Some(profile) = inner.profiles.remove(&tab) {
        profile.release();
    }
}

/// Start closing a tab's browser and return IMMEDIATELY, with its
/// surface already hidden.
///
/// The cookie flush that makes a close correct also makes it slow
/// enough to see, and what the user asked for was for the tab to go
/// away. So the pixels go synchronously and the flush finishes behind
/// them: by the time the strip has repainted without this tab, all
/// that is left is Chromium writing to disk.
///
/// Safe precisely because it does NOT outlive the window: if this was
/// the last tab, the window's own `CloseRequested` still finds the
/// browser live and defers destroying the HWND until the flush is
/// done. Nothing here can strand a half-written cookie store.
pub fn begin_close(app: &tauri::AppHandle, tab: u64) {
    crate::cef::hide(tab);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        close(&app, tab).await;
    });
}

/// Close every browser among `tabs`, concurrently — a window teardown
/// destroys all of its tabs at once, and serializing their (bounded,
/// flush-first) closes would multiply the worst case by the tab count.
pub async fn close_many(app: &tauri::AppHandle, tabs: &[u64]) {
    futures::future::join_all(tabs.iter().map(|&tab| close(app, tab))).await;
}

/// Every tab id that currently has a browser — the reconciler's orphan
/// sweep, which enumerates `app.webviews()` and therefore cannot see
/// these.
pub async fn live(app: &tauri::AppHandle) -> Vec<u64> {
    let browsers = app.state::<Browsers>();
    let inner = browsers.inner.lock().await;
    inner.spawned.iter().copied().collect()
}
