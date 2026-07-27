//! The shell's NATIVE half: windows and webviews. Every OS window is
//! a raw [`tauri::Window`] (label `shell-N` — ALL of them, none is
//! special; the boot window is just the first mint) hosting TWO chrome
//! webviews — `chrome-<window>` (the `index.html` entry, the tab strip,
//! pinned to the top band) and `status-<window>` (the `status.html`
//! entry, the bottom bar) — plus one CONTENT webview per tab
//! (`tab-<id>`, the `tab.html` entry) placed into the content rect
//! between them.
//!
//! The chrome is TWO band-sized webviews rather than one full-window
//! document because whatever spans the content band paints over it.
//! Between two WebView2 surfaces that is invisible (same
//! DirectComposition tree, ordered as expected); over a BROWSER tab,
//! whose surface is a plain child window CEF paints itself, it is
//! fatal — the compositor covers it regardless of HWND z-order. The
//! band belongs to the content alone.
//!
//! The active tab sits in the content rect, background tabs are
//! PARKED far offscreen but fully ALIVE and laid out (never
//! `hide()`n — see [`PARK_Y_LOGICAL`]; their streams and listeners
//! keep running — the old always-mounted CSS-hidden tabs, one
//! webview each).
//!
//! [`sync`] is the RECONCILER: it makes the native webview set match
//! the model, creating, reparenting (the lossless pop-out/pop-in
//! move), re-bounding, and showing/hiding as needed. It is
//! self-healing — a failed reparent degrades to close + recreate,
//! which is exactly the old rebuild behavior — and serialized by its
//! own `tokio::sync::Mutex`, taking a FRESH model view inside the
//! guard so a stale caller can never resurrect a closed webview.

use tauri::{Emitter, Manager};

use super::model::{ShellModel, Snapshot, Surface};

/// The theme's ground color (`--color-ground: #0c0a09` in app.css) —
/// painted behind every webview while its document boots, so neither
/// window creation nor tab creation ever flashes white.
const GROUND: tauri::webview::Color = tauri::webview::Color(0x0c, 0x0a, 0x09, 0xff);

/// The tab strip's height in LOGICAL pixels — the chrome's TabStrip
/// is styled to exactly this (`h-10`), and the docking hit-test
/// scales it by the target window's `scale_factor()`.
pub const STRIP_HEIGHT_LOGICAL: f64 = 40.0;

/// The status bar's height in LOGICAL pixels — the chrome's
/// StatusBar is styled to exactly this (`h-8`).
pub const STATUS_HEIGHT_LOGICAL: f64 = 32.0;

/// The reconciler's serialization guard (managed state).
#[derive(Default)]
pub struct WebviewSync(pub tokio::sync::Mutex<()>);

/// A content webview's label, from its tab id. Tab ids are never
/// reused, so labels never collide across a tab's whole life.
pub fn tab_label(id: u64) -> String {
    format!("tab-{id}")
}

/// A tab id, from a content webview's label. `None` = not a content
/// webview (chrome).
pub fn tab_id(label: &str) -> Option<u64> {
    label.strip_prefix("tab-")?.parse().ok()
}

/// A strip webview's label, from its window's label.
pub fn chrome_label(window: &str) -> String {
    format!("chrome-{window}")
}

/// A status-bar webview's label, from its window's label.
pub fn status_label(window: &str) -> String {
    format!("status-{window}")
}

/// The strip band (logical): the full width, `STRIP_HEIGHT_LOGICAL`
/// tall, at the top.
fn strip_rect(size: tauri::LogicalSize<f64>) -> tauri::Rect {
    tauri::Rect {
        position: tauri::LogicalPosition::new(0.0, 0.0).into(),
        size: tauri::LogicalSize::new(size.width, STRIP_HEIGHT_LOGICAL).into(),
    }
}

/// The status band (logical): the full width, `STATUS_HEIGHT_LOGICAL`
/// tall, pinned to the bottom.
fn status_rect(size: tauri::LogicalSize<f64>) -> tauri::Rect {
    tauri::Rect {
        position: tauri::LogicalPosition::new(
            0.0,
            (size.height - STATUS_HEIGHT_LOGICAL).max(0.0),
        )
        .into(),
        size: tauri::LogicalSize::new(size.width, STATUS_HEIGHT_LOGICAL).into(),
    }
}

/// A window's inner size in LOGICAL units.
fn logical_size(window: &tauri::Window) -> Option<tauri::LogicalSize<f64>> {
    let scale = window.scale_factor().ok()?;
    Some(window.inner_size().ok()?.to_logical::<f64>(scale))
}

/// The content rect (logical): the window minus the strip band and
/// the status bar.
fn content_rect(window: &tauri::Window) -> Option<tauri::Rect> {
    let scale = window.scale_factor().ok()?;
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    Some(tauri::Rect {
        position: tauri::LogicalPosition::new(0.0, STRIP_HEIGHT_LOGICAL).into(),
        size: tauri::LogicalSize::new(
            size.width,
            (size.height - STRIP_HEIGHT_LOGICAL - STATUS_HEIGHT_LOGICAL).max(1.0),
        )
        .into(),
    })
}

/// Background tabs are PARKED — full content-rect size, positioned
/// far outside the window — never `hide()`n: a hidden WebView2
/// suspends rendering before the document ever lays out at its real
/// bounds, so the first `show()` painted a stale tiny-viewport
/// layout (the "small width flicker"). A parked webview stays live
/// and correctly laid out at all times, and activation is a pure
/// MOVE — which cannot reflow. Parked-ness is always decided from
/// the MODEL, never inferred from webview geometry (a position()
/// heuristic mis-read child webviews and un-parked everything on
/// resize).
const PARK_Y_LOGICAL: f64 = 100_000.0;

/// `rect`, displaced to the parking position (same size).
fn parked(rect: &tauri::Rect) -> tauri::Rect {
    tauri::Rect {
        position: tauri::LogicalPosition::new(0.0, PARK_Y_LOGICAL).into(),
        size: rect.size,
    }
}

/// A logical rect in PHYSICAL pixels — what a native child window
/// (a browser tab's CEF surface) is positioned in. Tauri's own
/// `set_bounds` takes the logical rect directly; `SetWindowPos` does
/// not, and the difference is invisible at 100% scaling and glaring
/// anywhere else.
fn physical(rect: &tauri::Rect, scale: f64) -> (i32, i32, i32, i32) {
    let position = rect.position.to_logical::<f64>(scale);
    let size = rect.size.to_logical::<f64>(scale);
    (
        (position.x * scale).round() as i32,
        (position.y * scale).round() as i32,
        (size.width * scale).round() as i32,
        (size.height * scale).round() as i32,
    )
}

/// Build one shell window: a raw window plus its two CHROME webviews —
/// the tab strip pinned to the top band and the status bar to the
/// bottom, with the content band between them left EMPTY.
///
/// Two webviews rather than one full-window document, because whatever
/// spans the content band paints over it: harmless for a `tab-<id>`
/// webview (another WebView2, composited in the same
/// DirectComposition tree) and fatal for a browser tab, whose surface
/// is a plain child window CEF paints itself.
///
/// Neither uses `auto_resize`, which scales proportionally — right for
/// a full-window document and wrong for a fixed-height band. Both are
/// re-placed by [`layout_window`] on every resize, exactly like the
/// content webviews.
///
/// Content webviews are the reconciler's job, not this one's.
pub fn build_shell_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
    position: Option<tauri::LogicalPosition<f64>>,
) -> tauri::Result<tauri::Window> {
    let mut builder = tauri::window::WindowBuilder::new(app, label)
        .title(title)
        .inner_size(1024.0, 768.0);
    if let Some(position) = position {
        builder = builder.position(position.x, position.y);
    }
    let window = builder.build()?;
    let size = logical_size(&window)
        .unwrap_or_else(|| tauri::LogicalSize::new(1024.0, 768.0));
    for (label, entry, rect) in [
        (chrome_label(label), "index.html", strip_rect(size)),
        (status_label(label), "status.html", status_rect(size)),
    ] {
        window.add_child(
            tauri::webview::WebviewBuilder::new(
                label,
                tauri::WebviewUrl::App(entry.into()),
            )
            .background_color(GROUND)
            .initialization_script(super::CAPTURE_INIT_SCRIPT),
            rect.position,
            rect.size,
        )?;
    }
    Ok(window)
}

/// Make the native `tab-*` webview set match the model: close
/// orphans, create the missing, REPARENT the misplaced (lossless —
/// the webview's document never reloads), re-bound everything, show
/// the active tab and hide the rest, and push each hosting window's
/// UI state to its content webviews (`ui://changed`, targeted). NEVER
/// sets focus — an unrelated mutation must not steal it; the select
/// command focuses explicitly.
pub async fn sync(app: &tauri::AppHandle) {
    let sync_state = app.state::<WebviewSync>();
    let _guard = sync_state.0.lock().await;
    // The FRESH view, taken inside the guard: whatever mutation
    // prompted this call is already in the model, and a slower racing
    // caller can only ever apply something newer.
    let windows = app.state::<ShellModel>().windows_full().await;

    let alive = |id: u64| {
        windows
            .values()
            .any(|ws| ws.tabs.iter().any(|t| t.id == id))
    };

    // Orphans: a content webview whose tab is in no window died with
    // its tab (close is idempotent — the common case is the webview
    // already went down with its window's HWND).
    for (label, webview) in app.webviews() {
        if let Some(id) = tab_id(&label) {
            if !alive(id) {
                let _ = webview.close();
            }
        }
    }
    // The same sweep for browser tabs, which `app.webviews()` cannot
    // see: their surfaces belong to CEF, not Tauri, and closing one is
    // a flush-then-close round trip rather than a synchronous drop.
    let orphan_browsers: Vec<u64> = super::browser::live(app)
        .await
        .into_iter()
        .filter(|&id| !alive(id))
        .collect();
    super::browser::close_many(app, &orphan_browsers).await;

    for (win_label, ws) in &windows {
        // A window mid-build or mid-teardown: skip; the next sync
        // (every mutation issues one) retries.
        let Some(window) = app.get_window(win_label) else {
            continue;
        };
        let Some(rect) = content_rect(&window) else {
            continue;
        };
        for tab in &ws.tabs {
            let label = tab_label(tab.id);
            // Active = the content rect; background = parked (same
            // size, far offscreen — see PARK_Y_LOGICAL).
            let target = if tab.id == ws.active {
                rect
            } else {
                parked(&rect)
            };
            // A browser tab's surface is CEF's, not Tauri's: none of
            // the webview machinery below applies to it, from
            // `get_webview` through the `ui://changed` push (there is
            // no document listening — its zoom rides CEF's own API).
            if matches!(tab.kind.surface, Surface::Browser { .. }) {
                sync_browser(app, &window, tab, &target, tab.id == ws.active, &ws.ui)
                    .await;
                continue;
            }
            let webview = match app.get_webview(&label) {
                Some(webview) if webview.window().label() == win_label => Some(webview),
                Some(webview) => match webview.reparent(&window) {
                    // The lossless move: bare SetParent — the
                    // document, JS heap, and streams never notice.
                    Ok(()) => Some(webview),
                    // Self-heal: recreate below. Content state is
                    // lost, which is exactly the old rebuild-on-
                    // detach behavior — never worse.
                    Err(_) => {
                        let _ = webview.close();
                        None
                    }
                },
                None => None,
            };
            let webview = match webview {
                Some(webview) => webview,
                None => {
                    let builder = tauri::webview::WebviewBuilder::new(
                        &label,
                        tauri::WebviewUrl::App("tab.html".into()),
                    )
                    .focused(false)
                    .background_color(GROUND)
                    .initialization_script(super::CAPTURE_INIT_SCRIPT);
                    match window.add_child(builder, target.position, target.size) {
                        Ok(webview) => webview,
                        // Best-effort; the next sync retries.
                        Err(_) => continue,
                    }
                }
            };
            // Reparent does NOT reset bounds — always re-bound.
            let _ = webview.set_bounds(target);
            // Push the hosting window's UI state: adoption on create,
            // detach, and dock. (Targeted — the content listens with
            // a webview-scoped listener; its boot get covers the
            // race.)
            let _ = app.emit_to(label.as_str(), "ui://changed", &ws.ui);
        }
    }
}

/// One browser tab's leg of [`sync`]: create its CEF surface if it has
/// none, re-home it if it moved windows, and put it at `target`.
///
/// Deliberately WITHOUT the webview path's self-heal: a failed reparent
/// there degrades to close-and-recreate, which for a browser would
/// destroy the very session the feature exists to preserve. A browser
/// that cannot be re-homed stays where it is and the next sync retries.
async fn sync_browser(
    app: &tauri::AppHandle,
    window: &tauri::Window,
    tab: &super::model::Tab,
    target: &tauri::Rect,
    active: bool,
    ui: &super::model::UiState,
) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let (x, y, width, height) = physical(target, scale);
    // The parent HWND — a browser's surface is a native child window of
    // the shell window, not of any webview.
    #[cfg(target_os = "windows")]
    let parent = match window.hwnd() {
        Ok(hwnd) => hwnd.0 as isize,
        Err(_) => return,
    };
    #[cfg(not(target_os = "windows"))]
    let parent = 0isize;

    if !crate::cef::has_browser(tab.id) {
        // DETACHED, deliberately. The first browser tab on a machine
        // downloads ~200MB of Chromium, and this runs inside the
        // reconciler's serialization guard — awaiting it here would
        // freeze every tab operation in the app for the length of that
        // download. `spawn` claims its slot before doing anything slow,
        // so the syncs that land meanwhile are no-ops, and the browser
        // is created at the bounds passed here regardless of how much
        // later that is.
        let app = app.clone();
        let (id, title) = (tab.id, tab.title.clone());
        let identity = tab.kind.identity.clone();
        let surface = tab.kind.surface.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::browser::spawn(
                &app,
                id,
                &identity,
                &title,
                &surface,
                parent,
                (x, y, width, height),
            )
            .await
            {
                // The tab exists and stays in the strip; only its
                // surface is missing. Report it the way every other
                // shell failure is reported and let the user close it.
                super::report_shell(&app, "error", format!("browser tab {title}: {e}"))
                    .await;
            }
        });
        return;
    }
    // Re-home first (a no-op when it never moved — SetParent to the
    // current parent is legal), then bound: reparenting does not
    // preserve position any more than a webview reparent does.
    let _ = crate::cef::reparent(tab.id, parent);
    crate::cef::set_bounds(tab.id, x, y, width, height);
    // The chrome webview spans the whole window, so an active browser
    // has to be re-asserted above it. (Parked tabs are far offscreen —
    // z-order is meaningless there.)
    if active {
        crate::cef::raise(tab.id);
    }
    // The window's zoom, in CEF's units: a level, where each step is a
    // factor of 1.2 and 0 is 100%.
    crate::cef::set_zoom(tab.id, ui.zoom.max(0.01).ln() / 1.2f64.ln());
}

/// Resize one window's content webviews (Resized /
/// ScaleFactorChanged). SIZE ONLY: the active position (0, strip)
/// and the parked position (0, PARK_Y) are both constants, so a
/// resize never needs to know who is active — placement belongs to
/// [`sync`] alone. Synchronous and inline on the main thread (the
/// event closure), where set_size takes the dispatch fast path —
/// no locks, no spawned tasks, no round trips.
pub fn layout_window(app: &tauri::AppHandle, label: &str) {
    let Some(window) = app.get_window(label) else {
        return;
    };
    let Some(rect) = content_rect(&window) else {
        return;
    };
    for webview in window.webviews() {
        if tab_id(webview.label()).is_some() {
            let _ = webview.set_size(rect.size);
        }
    }
    // The two chrome bands. Unlike the content webviews these need
    // their POSITION too — the status bar is pinned to the bottom, so
    // its y moves with every resize — which is exactly what
    // `auto_resize` could not express and why neither uses it.
    if let Some(size) = logical_size(&window) {
        for (label, band) in [
            (chrome_label(label), strip_rect(size)),
            (status_label(label), status_rect(size)),
        ] {
            if let Some(webview) = app.get_webview(&label) {
                let _ = webview.set_bounds(band);
            }
        }
    }
    // Browser tabs are not in `window.webviews()`, and their surfaces
    // take PHYSICAL bounds. Same size-only spirit: CEF remembers each
    // browser's parent and its last y (active or parked — both
    // constants a resize never changes), so this stays a pure
    // reposition with no model read, exactly like the loop above.
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let scale = window.scale_factor().unwrap_or(1.0);
        let (x, _, width, height) = physical(&rect, scale);
        crate::cef::relayout(hwnd.0 as isize, x, width, height);
    }
}

/// Broadcast the snapshot to every chrome + retitle `touched`
/// windows. Call with NO model guard held.
pub fn publish(app: &tauri::AppHandle, snapshot: &Snapshot, touched: &[String]) {
    // Best-effort everywhere: a window mid-teardown must never turn a
    // mutation into an error.
    let _ = app.emit("tabs://changed", snapshot);
    for label in touched {
        sync_title(app, snapshot, label);
    }
}

/// A window's title follows its ACTIVE tab as `<identity> - <name>`
/// (product name when it has none) — every window alike; none is
/// special.
pub fn sync_title(app: &tauri::AppHandle, snapshot: &Snapshot, label: &str) {
    let Some(window) = app.get_window(label) else {
        return;
    };
    let title = snapshot
        .windows
        .get(label)
        .and_then(|wt| wt.tabs.iter().find(|t| t.id == wt.active))
        .map(|t| format!("{} - {}", t.kind.identity, t.title))
        .unwrap_or_else(|| "ObjectiveAI Viewer".to_string());
    let _ = window.set_title(&title);
}
