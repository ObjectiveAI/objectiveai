//! The shell's NATIVE half: windows and webviews. Every OS window is
//! a raw [`tauri::Window`] (label `shell-N` — ALL of them, none is
//! special; the boot window is just the first mint) hosting exactly
//! one CHROME webview (`chrome-<window>`, the `index.html` entry —
//! tab strip + status bar, full-window with proportional auto-resize)
//! plus one CONTENT webview per tab (`tab-<id>`, the `tab.html`
//! entry) placed into the content rect between the strip and the
//! status bar. Content webviews composite ABOVE the chrome's middle;
//! the active tab is shown, background tabs are hidden but ALIVE
//! (their streams and listeners keep running — the old always-mounted
//! CSS-hidden tabs, one webview each).
//!
//! [`sync`] is the RECONCILER: it makes the native webview set match
//! the model, creating, reparenting (the lossless pop-out/pop-in
//! move), re-bounding, and showing/hiding as needed. It is
//! self-healing — a failed reparent degrades to close + recreate,
//! which is exactly the old rebuild behavior — and serialized by its
//! own `tokio::sync::Mutex`, taking a FRESH model view inside the
//! guard so a stale caller can never resurrect a closed webview.

use tauri::{Emitter, Manager};

use super::model::{ShellModel, Snapshot};

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

/// A chrome webview's label, from its window's label.
pub fn chrome_label(window: &str) -> String {
    format!("chrome-{window}")
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

/// Build one shell window: a raw window + its chrome webview
/// (full-window, proportional auto-resize — full-window IS
/// expressible as 1.0 rates, unlike the fixed-band content rect).
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
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window
        .inner_size()
        .map(|s| s.to_logical::<f64>(scale))
        .unwrap_or_else(|_| tauri::LogicalSize::new(1024.0, 768.0));
    window.add_child(
        tauri::webview::WebviewBuilder::new(
            chrome_label(label),
            tauri::WebviewUrl::App("index.html".into()),
        )
        .auto_resize()
        .background_color(GROUND)
        .initialization_script(super::CAPTURE_INIT_SCRIPT),
        tauri::LogicalPosition::new(0.0, 0.0),
        size,
    )?;
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

    // Orphans: a content webview whose tab is in no window died with
    // its tab (close is idempotent — the common case is the webview
    // already went down with its window's HWND).
    for (label, webview) in app.webviews() {
        if let Some(id) = tab_id(&label) {
            let live = windows
                .values()
                .any(|ws| ws.tabs.iter().any(|t| t.id == id));
            if !live {
                let _ = webview.close();
            }
        }
    }

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
                    match window.add_child(builder, rect.position, rect.size) {
                        Ok(webview) => webview,
                        // Best-effort; the next sync retries.
                        Err(_) => continue,
                    }
                }
            };
            // Reparent does NOT reset bounds — always re-bound.
            let _ = webview.set_bounds(rect);
            if tab.id == ws.active {
                let _ = webview.show();
            } else {
                let _ = webview.hide();
            }
            // Push the hosting window's UI state: adoption on create,
            // detach, and dock. (Targeted — the content listens with
            // a webview-scoped listener; its boot get covers the
            // race.)
            let _ = app.emit_to(label.as_str(), "ui://changed", &ws.ui);
        }
    }
}

/// Re-bound one window's content webviews (Resized /
/// ScaleFactorChanged — high-frequency, so no model lock and no
/// reconcile: purely a relayout of whatever is already placed).
pub fn layout_window(app: &tauri::AppHandle, label: &str) {
    let Some(window) = app.get_window(label) else {
        return;
    };
    let Some(rect) = content_rect(&window) else {
        return;
    };
    for webview in window.webviews() {
        if tab_id(webview.label()).is_some() {
            let _ = webview.set_bounds(rect);
        }
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
