//! The reattach half of the shell: dragging a shell window over
//! another window's TAB STRIP docks it — every tab it carries merges
//! into the target (their content webviews REPARENT across — nothing
//! reloads) and the source window closes (Chrome semantics).
//!
//! There is no native drag-end event (tao does not surface
//! `WM_EXITSIZEMOVE`), so the dock commits on a heuristic that is
//! made SAFE on Windows by a real button check:
//!
//! 1. Every `Moved` event feeds a per-label debounce (~200ms).
//! 2. When a window goes quiet, the mouse button state is read
//!    (`GetAsyncKeyState(VK_LBUTTON)`): still held ⇒ the user merely
//!    PAUSED mid-drag — re-arm and wait. Committing while the OS
//!    modal move loop still owns the window would destroy the window
//!    mid-drag (input-state corruption). Non-Windows falls back to
//!    debounce-only.
//! 3. Quiet + button up ⇒ if the cursor sits inside another window's
//!    strip band (top [`STRIP_HEIGHT_LOGICAL`] logical px of its
//!    client area, scaled by THAT window's DPI — all math in physical
//!    screen coordinates, so mixed-DPI monitors are safe), merge.
//!
//! ANY window can be a source and any window a target — no window is
//! special (the drag gate above keeps ordinary repositioning from
//! docking: only a real mouse drag released over a strip commits).
//! Minimized/mid-destroy windows are skipped. Multiple band hits
//! (overlapping windows — no z-order API exists) prefer the focused
//! window. During movement a throttled `tabs://dock-preview` event
//! highlights the hovered strip; a trailing clear makes sure no
//! highlight ever sticks.

use tauri::{Emitter, Manager};

use super::model::ShellModel;
use super::native::{self, STRIP_HEIGHT_LOGICAL};

/// Quiet time after the last `Moved` before a dock is considered.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(200);

/// Is the primary mouse button currently held? Windows: the real
/// answer; elsewhere: `false` (debounce-only commit).
fn primary_button_down() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LBUTTON,
        };
        // High bit set = currently down.
        (unsafe { GetAsyncKeyState(VK_LBUTTON as i32) } as u16) & 0x8000 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// The window label (≠ `moving`, alive, not minimized) whose strip
/// band contains the physical `cursor`, preferring the focused one on
/// overlap.
fn strip_hit(
    app: &tauri::AppHandle,
    moving: &str,
    cursor: tauri::PhysicalPosition<f64>,
) -> Option<String> {
    let mut hit: Option<(String, bool)> = None;
    for (label, window) in app.windows() {
        if label == moving {
            continue;
        }
        if window.is_minimized().unwrap_or(true) {
            continue;
        }
        let Ok(pos) = window.inner_position() else {
            continue;
        };
        let Ok(size) = window.inner_size() else {
            continue;
        };
        let scale = window.scale_factor().unwrap_or(1.0);
        let band = STRIP_HEIGHT_LOGICAL * scale;
        let inside = cursor.x >= pos.x as f64
            && cursor.x < pos.x as f64 + size.width as f64
            && cursor.y >= pos.y as f64
            && cursor.y < pos.y as f64 + band;
        if !inside {
            continue;
        }
        let focused = window.is_focused().unwrap_or(false);
        match &hit {
            Some((_, true)) => {}
            _ => hit = Some((label.clone(), focused)),
        }
        if focused {
            hit = Some((label, true));
        }
    }
    hit.map(|(label, _)| label)
}

/// Spawn the docking task. `run_return`'s closure feeds it every
/// `Moved` label through the paired sender.
pub fn spawn_docking(
    app: tauri::AppHandle,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    tauri::async_runtime::spawn(async move {
        // The window currently "in flight" — a Moved from a different
        // window simply retargets (one drag at a time is the physical
        // reality). `previewed` = the strip currently highlighted.
        let mut previewed: Option<String> = None;
        loop {
            let Some(mut label) = rx.recv().await else {
                return;
            };
            // A dock requires an actual mouse DRAG: remember whether
            // the button was ever observed down during this gesture,
            // so keyboard-driven moves (Win+arrow snap, programmatic
            // repositioning) can never dock a window. On non-Windows
            // (no button API) this stays true — debounce-only.
            let mut was_drag = cfg!(not(windows)) || primary_button_down();
            // One GESTURE: alternate between swallowing Moved events
            // (updating the preview) and, on each SETTLE of quiet,
            // checking the button — held means a mid-drag pause (keep
            // waiting; the timeout doubles as the poll timer), up
            // means the drag ended (commit-check once, done).
            loop {
                match tokio::time::timeout(SETTLE, rx.recv()).await {
                    Ok(Some(next)) => {
                        label = next;
                        was_drag = was_drag || primary_button_down();
                        let target = app
                            .cursor_position()
                            .ok()
                            .and_then(|c| strip_hit(&app, &label, c));
                        if target != previewed {
                            if let Some(old) = &previewed {
                                let _ = app.emit_to(
                                    native::chrome_label(old).as_str(),
                                    "tabs://dock-preview",
                                    false,
                                );
                            }
                            if let Some(new) = &target {
                                let _ = app.emit_to(
                                    native::chrome_label(new).as_str(),
                                    "tabs://dock-preview",
                                    true,
                                );
                            }
                            previewed = target;
                        }
                    }
                    Ok(None) => return,
                    Err(_) => {
                        if primary_button_down() {
                            // Paused mid-drag — never commit while the
                            // OS modal move loop may still own the
                            // window; keep waiting.
                            continue;
                        }
                        // Quiet + button up = the drag is over.
                        if was_drag {
                            if let Ok(cursor) = app.cursor_position() {
                                if let Some(target) =
                                    strip_hit(&app, &label, cursor)
                                {
                                    dock(&app, &label, &target).await;
                                }
                            }
                        }
                        break;
                    }
                }
            }
            // Gesture resolved — no highlight may outlive it.
            if let Some(old) = previewed.take() {
                let _ = app.emit_to(
                    native::chrome_label(&old).as_str(),
                    "tabs://dock-preview",
                    false,
                );
            }
        }
    });
}

/// Merge `source`'s tabs into `target` and close `source`. All
/// best-effort — a window that vanished mid-flight makes this a
/// no-op. Ordering matters: the reconciler REPARENTS the source's
/// content webviews into the target BEFORE the source window closes
/// (they must be out before the source HWND — and everything childed
/// to it — dies).
async fn dock(app: &tauri::AppHandle, source: &str, target: &str) {
    let model = app.state::<ShellModel>();
    let Some(snapshot) = model.merge_windows(source, target).await else {
        return;
    };
    // A dock is user arrangement too (unreachable without a prior
    // detach, which already set this — free insurance).
    app.state::<super::TabInventory>().set_user_controlled();
    native::publish(app, &snapshot, &[target.to_string()]);
    native::sync(app).await;
    if let Some(window) = app.get_window(target) {
        let _ = window.set_focus();
    }
    // Park keyboard focus INSIDE the active tab's webview — leaving
    // it on the window HWND makes the next click into any child
    // webview get eaten moving focus in (the post-dock "first click
    // does nothing"); select semantics put it here anyway.
    if let Some(active) = snapshot.windows.get(target).map(|ws| ws.active) {
        if active != 0 {
            if let Some(webview) = app.get_webview(&native::tab_label(active)) {
                let _ = webview.set_focus();
            }
            // Same for a browser tab, whose surface is CEF's and has no
            // webview to focus. No-op for every component tab.
            crate::cef::focus(active);
        }
    }
    if let Some(window) = app.get_window(source) {
        let _ = window.close();
    }
}
