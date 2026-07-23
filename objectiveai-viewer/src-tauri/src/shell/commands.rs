//! The Tauri commands the chrome webviews drive the shell with. Every
//! mutating command follows one shape: mutate the model (its guard
//! dropped on return) → [`publish`](super::native::publish) the
//! snapshot → [`sync`](super::native::sync) the native webviews.
//!
//! Commands take the calling [`tauri::Webview`] — never a
//! `WebviewWindow`, whose extractor ERRORS on any multi-webview
//! window — and resolve the caller's window through it, so a content
//! webview invoking `tabs_open` attributes to its CURRENT hosting
//! window even right after a reparent.

use tauri::Manager;

use super::model::{ShellModel, Snapshot, TabKind, UiState};
use super::native;

/// The model snapshot — a chrome's boot read. (Subscribe to
/// `tabs://changed` FIRST, then snapshot; apply either only when the
/// generation advances.)
#[tauri::command]
pub async fn tabs_snapshot(
    model: tauri::State<'_, ShellModel>,
) -> Result<Snapshot, String> {
    Ok(model.snapshot().await)
}

/// Open `kind`: if a tab with this exact kind exists ANYWHERE,
/// activate + focus its window (open-or-focus, like the old bespoke
/// windows); otherwise append a fresh tab to the CALLER's window and
/// activate it.
#[tauri::command]
pub async fn tabs_open(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    kind: TabKind,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();
    let opened = model
        .open_or_focus(&caller, kind, |label| app.get_window(label).is_some())
        .await;
    native::publish(&app, &opened.snapshot, &opened.touched);
    native::sync(&app).await;
    if let Some(label) = opened.focus {
        if let Some(target) = app.get_window(&label) {
            let _ = target.set_focus();
        }
    }
    Ok(())
}

/// Activate a tab in the calling window, and hand it keyboard focus.
/// Unknown ids no-op.
#[tauri::command]
pub async fn tabs_select(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    tab_id: u64,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();
    let Some(snapshot) = model.select(&caller, tab_id).await else {
        return Ok(());
    };
    native::publish(&app, &snapshot, &[caller]);
    native::sync(&app).await;
    // Focus follows an EXPLICIT selection (the reconciler itself
    // never focuses — an unrelated mutation must not steal it).
    if let Some(webview) = app.get_webview(&native::tab_label(tab_id)) {
        let _ = webview.set_focus();
    }
    Ok(())
}

/// Close a tab (idempotent). A SHELL window whose last tab closes is
/// itself closed; the main window never auto-closes (it renders an
/// empty state instead).
#[tauri::command]
pub async fn tabs_close(
    app: tauri::AppHandle,
    model: tauri::State<'_, ShellModel>,
    tab_id: u64,
) -> Result<(), String> {
    let Some(closed) = model.close(tab_id).await else {
        return Ok(());
    };
    native::publish(&app, &closed.snapshot, &closed.touched);
    native::sync(&app).await;
    if let Some(label) = closed.close_window {
        if let Some(window) = app.get_window(&label) {
            let _ = window.close();
        }
    }
    Ok(())
}

/// Reorder a tab WITHIN the calling window (cross-window moves ride
/// the dock flow, not this). `index` is clamped. Purely a model/strip
/// affair — no native webview changes.
#[tauri::command]
pub async fn tabs_move(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    tab_id: u64,
    index: usize,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();
    let Some(snapshot) = model.move_tab(&caller, tab_id, index).await else {
        return Ok(());
    };
    native::publish(&app, &snapshot, &[caller]);
    Ok(())
}

/// Tear `tab_id` out of the calling window into a FRESH shell window
/// under the cursor, then hand the user's still-held drag to the OS
/// (`start_dragging`). The tab's content webview RIDES ALONG — a
/// reparent, not a rebuild: nothing in it reloads. Idempotent per tab
/// (a second racing call finds the tab already moved and no-ops). A
/// 1-tab shell window skips the pointless rebuild: the whole window
/// IS the tab — just start dragging it.
#[tauri::command]
pub async fn tabs_detach(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    tab_id: u64,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();

    // 1-tab shell: drag the source window itself.
    if caller != "main" && model.is_sole_tab(&caller, tab_id).await {
        let _ = webview.window().start_dragging();
        return Ok(());
    }

    // Placement BEFORE the (slow) window build; re-anchored again just
    // before start_dragging, since the cursor keeps moving meanwhile.
    let cursor = app.cursor_position().ok();

    // Move the tab into a freshly minted shell entry FIRST — the new
    // window's chrome boot snapshot must find its tab.
    let Some(detached) = model.detach_to(&caller, tab_id).await else {
        // Already moved by a racing detach — idempotent no-op.
        return Ok(());
    };
    native::publish(&app, &detached.snapshot, &detached.touched);

    let position = cursor.map(|cursor| {
        // Grab point ≈ inside the new window's strip, a bit in from
        // the corner. Physical cursor → logical position via the
        // SOURCE window's scale (best available guess pre-build).
        let scale = webview.window().scale_factor().unwrap_or(1.0);
        tauri::LogicalPosition::new(cursor.x / scale - 60.0, cursor.y / scale - 20.0)
    });
    let new_window =
        match native::build_shell_window(&app, &detached.label, &detached.title, position) {
            Ok(window) => window,
            Err(e) => {
                // Roll back: the tab returns to the source window, the
                // orphan entry dies.
                let snapshot = model.rollback_detach(&detached.label, &caller).await;
                native::publish(&app, &snapshot, &[caller]);
                native::sync(&app).await;
                return Err(format!("detach: window build failed: {e}"));
            }
        };
    // THE lossless moment: the reconciler reparents the tab's live
    // content webview into the new window.
    native::sync(&app).await;

    // Re-anchor at the CURRENT cursor (it moved during the build),
    // then hand the still-held drag to the OS. Both best-effort: a
    // released button just leaves the window placed where it is, and
    // tao's ReleaseCapture can Err benignly.
    if let Ok(cursor) = app.cursor_position() {
        let scale = new_window.scale_factor().unwrap_or(1.0);
        let _ = new_window.set_position(tauri::LogicalPosition::new(
            cursor.x / scale - 60.0,
            cursor.y / scale - 20.0,
        ));
    }
    let _ = new_window.start_dragging();
    Ok(())
}

/// Merge UI fields (zoom / orientation) into the calling window's
/// state and push the result to its content webviews. Chrome owns
/// the controls; content adopts whatever its CURRENT window says.
#[tauri::command]
pub async fn ui_set(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    zoom: Option<f64>,
    orientation: Option<String>,
) -> Result<(), String> {
    let window = webview.window().label().to_string();
    let Some((ui, tab_ids)) = model.set_ui(&window, zoom, orientation).await else {
        return Ok(());
    };
    use tauri::Emitter;
    for id in tab_ids {
        let _ = app.emit_to(native::tab_label(id).as_str(), "ui://changed", &ui);
    }
    Ok(())
}

/// The calling webview's window's UI state — a content webview's boot
/// read (listen webview-scoped FIRST, then get). `window()` tracks
/// reparents on the Rust side, so this is correct even right after a
/// pop-out/pop-in.
#[tauri::command]
pub async fn ui_get(
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
) -> Result<UiState, String> {
    Ok(model.ui_for_window(webview.window().label()).await)
}
