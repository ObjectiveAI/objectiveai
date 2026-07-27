//! CEF in the viewer process: initialization, the browser registry,
//! and every operation a browser tab's surface needs.
//!
//! **Process model.** Chromium is multi-process, and on Windows/Linux
//! it re-executes THIS binary for each helper (renderer, GPU, utility,
//! ...) with `--type=<kind>` on the command line. [`is_helper_process`]
//! screens for that as the very first statement of `main`, before the
//! tokio runtime is even built — a helper that fell through into the
//! viewer's startup would try to be a second viewer, and (worse) would
//! print the daemon's readiness line, which the daemon is blocked
//! reading.
//!
//! In the browser process CEF is initialized LAZILY, on the first
//! browser tab, so a viewer that never opens one never needs a Chromium
//! runtime on disk at all (see [`super::install`]).
//! `multi_threaded_message_loop` gives CEF its own UI thread instead of
//! the main one, which Tauri's event loop already owns.
//!
//! **The two-phase close.** CEF writes cookies lazily: a plain close
//! drops a session created seconds earlier, and the loss is invisible —
//! the browser closes normally and the next open is simply signed out.
//! [`LifeSpan::do_close`] therefore CANCELS the first close, flushes the
//! cookie store, and re-issues the close from the flush's completion
//! callback; the second pass sees the flag and lets it through. Every
//! caller that closes a browser tab waits for `on_before_close` (with a
//! bound, so a wedged renderer can't hang a tab close forever).
//!
//! **Per browser, not per process.** Everything psyops kept in a
//! process-wide `OnceLock<Mutex<Option<Browser>>>` is keyed by tab id
//! here: each browser gets its own `Client`, its own handlers carrying
//! that tab id, and its own flushed-flag. Tab ids are never reused, so
//! a stale entry can never be mistaken for a live one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use cef::rc::Rc as _;
use cef::sys::cef_window_handle_t;
use cef::*;

/// The environment variable carrying the resolved Chromium runtime
/// directory to helper subprocesses. A helper cannot fetch the runtime
/// (it has no config, no daemon, and no business downloading 200MB), so
/// the browser process — which already resolved it — passes it down.
/// Child processes inherit the environment, so setting it once before
/// `initialize` covers every helper CEF ever spawns.
const RUNTIME_DIR_ENV: &str = "OBJECTIVEAI_CEF_DIR";

/// Set once CEF is initialized. Also the "is CEF up?" flag — every
/// operation below is a no-op before it.
static INITIALIZED: OnceLock<()> = OnceLock::new();

/// One live browser, keyed by the tab id that owns it.
struct Entry {
    browser: Browser,
    /// The CEF child window handle, for `SetWindowPos` reflows. CEF
    /// parents its own window under the one we passed at create time;
    /// this is that inner window, not ours.
    child: isize,
}

/// Live browsers by tab id.
static BROWSERS: OnceLock<Mutex<HashMap<u64, Entry>>> = OnceLock::new();

/// Where each browser last was: its parent window handle and its last
/// applied `y`. Kept apart from [`Entry`] because it is written from
/// the moment a browser is REQUESTED, whereas the entry only exists
/// once CEF has asynchronously created it — and a window resize can
/// land in that gap.
///
/// This is what lets [`relayout`] reposition a window's browsers
/// without consulting the shell model: the y it must preserve
/// (content-rect or parked, both constants a resize never changes) is
/// already recorded here.
static LAYOUT: OnceLock<Mutex<HashMap<u64, Placement>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct Placement {
    parent: isize,
    y: i32,
}

fn layout() -> &'static Mutex<HashMap<u64, Placement>> {
    LAYOUT.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Parked waiters for `on_before_close`, by tab id. A close may be
/// awaited by more than one caller (the tab close and the window
/// teardown can race), so this is a list, not a slot.
static CLOSE_WAITERS: OnceLock<Mutex<HashMap<u64, Vec<tokio::sync::oneshot::Sender<()>>>>> =
    OnceLock::new();

fn browsers() -> &'static Mutex<HashMap<u64, Entry>> {
    BROWSERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn close_waiters()
-> &'static Mutex<HashMap<u64, Vec<tokio::sync::oneshot::Sender<()>>>> {
    CLOSE_WAITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Is CEF up in this process?
pub fn is_initialized() -> bool {
    INITIALIZED.get().is_some()
}

/// Does tab `id` have a live browser?
pub fn has_browser(id: u64) -> bool {
    browsers()
        .lock()
        .map(|m| m.contains_key(&id))
        .unwrap_or(false)
}

/// The `Browser` for tab `id`, cloned (which bumps its CEF refcount, so
/// the handle outlives the registry lock).
fn browser(id: u64) -> Option<Browser> {
    browsers().lock().ok()?.get(&id).map(|e| e.browser.clone())
}

// ---------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------

/// Convert a path to a [`CefString`] CEF will accept.
///
/// On Windows, Chromium's `chrome_browser_context.cc` rejects paths
/// that mix `/` and `\` — "Cannot create profile at path ..." — and
/// mixing is easy to produce, since `PathBuf::join` preserves whatever
/// separator each half already had. An `OBJECTIVEAI_DIR` that came from
/// a shell (`$HOME/.objectiveai`, forward slashes) joined to
/// backslashed segments yields exactly that. Normalize here, once, at
/// the boundary.
fn cef_path(path: &Path) -> CefString {
    let s = path.to_string_lossy();
    #[cfg(target_os = "windows")]
    {
        CefString::from(s.replace('/', "\\").as_str())
    }
    #[cfg(not(target_os = "windows"))]
    {
        CefString::from(s.as_ref())
    }
}

// ---------------------------------------------------------------------
// Process entry points
// ---------------------------------------------------------------------

/// Is this process a Chromium helper (renderer / GPU / utility / ...)
/// rather than the viewer?
///
/// Must be answered BEFORE anything else in `main` — a helper inherits
/// none of the viewer's expectations, and any startup work done here is
/// work done in every one of Chromium's several subprocesses.
///
/// macOS helpers are separate `.app` bundles rather than re-invocations
/// of this binary, so this is always false there.
pub fn is_helper_process() -> bool {
    #[cfg(target_os = "macos")]
    {
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::args().any(|arg| arg.starts_with("--type="))
    }
}

/// Run as a Chromium helper subprocess, then exit. Never returns.
///
/// Called only when [`is_helper_process`] said so. Note what this does
/// NOT do: print the readiness line. That line is the daemon's signal
/// that its viewer child is up, and a helper printing it would tell the
/// daemon a browser tab's renderer is the viewer.
#[cfg(not(target_os = "macos"))]
pub fn run_helper_and_exit() -> ! {
    // The runtime is wherever the browser process found it — a helper
    // has no way to resolve (let alone download) it for itself.
    if let Some(dir) = std::env::var_os(RUNTIME_DIR_ENV) {
        super::add_dll_directory(Path::new(&dir));
    }
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = args::Args::new();
    let main_args = args.as_main_args();
    // An `App` MUST be passed, not `None`: the renderer builds its own
    // scheme registry from `on_register_custom_schemes`, and a renderer
    // that never saw ours rejects those URLs as unsupported even though
    // the browser side handles them.
    let mut app = ViewerApp::new();
    let _ = execute_process(Some(main_args), Some(&mut app), std::ptr::null_mut());
    std::process::exit(0);
}

/// Bring CEF up. Idempotent — later calls are no-ops.
///
/// MUST run on the main thread (CEF's requirement), and MUST run before
/// any other call in this module. `runtime_dir` holds `libcef.dll` and
/// the resource paks; `root_cache` is the profiles root, which CEF locks
/// for the process lifetime.
///
/// Every path CEF is given is set EXPLICITLY, because the runtime is
/// usually not beside the executable — CEF's defaults all assume it is.
pub fn initialize(runtime_dir: &Path, root_cache: &Path) -> Result<(), String> {
    if is_initialized() {
        return Ok(());
    }

    // macOS owns the main-thread run loop through NSApplication, so
    // `multi_threaded_message_loop` is unavailable and CEF has to share
    // it via `external_message_pump` — plus a `.app` layout with a
    // separate helper bundle. Neither is built yet; refuse clearly
    // rather than initialize into a deadlock.
    #[cfg(target_os = "macos")]
    {
        let _ = (runtime_dir, root_cache);
        return Err(
            "browser tabs are not supported on macOS yet — CEF needs the \
             external-message-pump + helper-bundle layout"
                .to_string(),
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        initialize_inner(runtime_dir, root_cache)
    }
}

#[cfg(not(target_os = "macos"))]
fn initialize_inner(runtime_dir: &Path, root_cache: &Path) -> Result<(), String> {
    super::add_dll_directory(runtime_dir);
    // Inherited by every helper CEF spawns — see [`RUNTIME_DIR_ENV`].
    // SAFETY: single-threaded with respect to other env mutation — this
    // runs on the main thread before CEF exists, and nothing else in
    // the viewer writes the environment.
    unsafe {
        std::env::set_var(RUNTIME_DIR_ENV, runtime_dir);
    }

    std::fs::create_dir_all(root_cache)
        .map_err(|e| format!("create browsers root {root_cache:?}: {e}"))?;

    // The API version handshake, before any other CEF call.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let cef_args = args::Args::new();
    let main_args = cef_args.as_main_args();

    // In the browser process this recognizes the absence of `--type=`
    // and returns -1 so we can proceed. Anything else means helper
    // screening failed and we are about to initialize inside a
    // subprocess.
    let ret = execute_process(Some(main_args), None, std::ptr::null_mut());
    if ret != -1 {
        return Err(format!(
            "cef execute_process returned {ret} in the browser process — \
             helper detection did not run first"
        ));
    }

    let exe = std::env::current_exe()
        .map_err(|e| format!("current_exe: {e}"))?;
    let log_file = root_cache.join("cef-debug.log");

    let settings = Settings {
        // CEF runs its own UI thread; Tauri's wry/tao event loop keeps
        // the main one.
        multi_threaded_message_loop: 1,
        no_sandbox: 1,
        root_cache_path: cef_path(root_cache),
        log_file: cef_path(&log_file),
        log_severity: LogSeverity::INFO,
        // The three that a not-beside-the-exe runtime makes mandatory.
        resources_dir_path: cef_path(runtime_dir),
        locales_dir_path: cef_path(&runtime_dir.join("locales")),
        browser_subprocess_path: cef_path(&exe),
        ..Default::default()
    };

    let mut app = ViewerApp::new();
    let init = cef::initialize(
        Some(main_args),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    );
    if init != 1 {
        return Err(format!(
            "cef initialize failed — see {}",
            log_file.display()
        ));
    }
    let _ = INITIALIZED.set(());
    Ok(())
}

/// Tear CEF down. Every browser must already be closed — the shell
/// closes each browser tab (and waits for it) before its window dies.
pub fn shutdown() {
    if is_initialized() {
        cef::shutdown();
    }
}

// ---------------------------------------------------------------------
// Window handles
// ---------------------------------------------------------------------

/// Wrap a raw platform handle in the platform's `cef_window_handle_t`.
///
/// On Windows that alias resolves to `cef::sys::HWND`, a bindgen tuple
/// struct — and a tuple-struct constructor cannot be named through a
/// type alias, so the underlying struct has to be named explicitly.
#[cfg(target_os = "windows")]
fn handle_from_raw(raw: isize) -> cef_window_handle_t {
    cef::sys::HWND(raw as *mut cef::sys::HWND__)
}

#[cfg(not(target_os = "windows"))]
fn handle_from_raw(raw: isize) -> cef_window_handle_t {
    raw as cef_window_handle_t
}

#[cfg(target_os = "windows")]
fn raw_from_handle(h: cef_window_handle_t) -> isize {
    h.0 as isize
}

#[cfg(not(target_os = "windows"))]
fn raw_from_handle(h: cef_window_handle_t) -> isize {
    h as isize
}

// ---------------------------------------------------------------------
// Browser operations
// ---------------------------------------------------------------------

/// Everything one browser tab needs at create time.
pub struct Spawn {
    /// The owning tab's id — the registry key, and what every handler
    /// carries so it can find its way back here.
    pub tab: u64,
    /// The parent window's native handle (HWND on Windows).
    pub parent: isize,
    /// Physical-pixel bounds inside the parent.
    pub bounds: (i32, i32, i32, i32),
    /// The profile directory, or `None` for an entirely in-memory
    /// browser. `None` is the natural default: no `cache_path` means
    /// CEF keeps cookies, storage and cache in memory and drops them
    /// with the browser.
    pub cache: Option<PathBuf>,
    pub url: String,
    /// JavaScript injected into every main-frame load — already
    /// resolved from the plugin's manifest and read off disk.
    pub script: Option<String>,
}

/// Create the browser for a tab. Fire-and-forget: CEF creates
/// asynchronously and `on_after_created` registers it.
pub fn create_browser(spawn: Spawn) -> Result<(), String> {
    if !is_initialized() {
        return Err("cef is not initialized".to_string());
    }
    let (x, y, width, height) = spawn.bounds;
    let bounds = Rect {
        x,
        y,
        width: width.max(1),
        height: height.max(1),
    };
    let window_info = WindowInfo {
        runtime_style: RuntimeStyle::ALLOY,
        ..Default::default()
    }
    .set_as_child(handle_from_raw(spawn.parent), &bounds);

    // The per-profile RequestContext. Its `cache_path` must be an
    // IMMEDIATE child of the `root_cache_path` given at initialize
    // time — see [`super::ProfileRoot`] for what nesting costs.
    let mut request_context = match &spawn.cache {
        Some(dir) => {
            let ctx_settings = RequestContextSettings {
                cache_path: cef_path(dir),
                persist_session_cookies: 1,
                ..Default::default()
            };
            request_context_create_context(Some(&ctx_settings), None)
        }
        // No cache path at all — CEF gives us an in-memory context.
        None => request_context_create_context(None, None),
    }
    .ok_or_else(|| "cef request_context_create_context returned null".to_string())?;

    if let Ok(mut placements) = layout().lock() {
        placements.insert(
            spawn.tab,
            Placement {
                parent: spawn.parent,
                y,
            },
        );
    }

    let flushed = Arc::new(AtomicBool::new(false));
    let mut client = ViewerClient::new(spawn.tab, flushed, spawn.script);
    let url = CefString::from(spawn.url.as_str());
    let settings = BrowserSettings::default();
    let created = browser_host_create_browser(
        Some(&window_info),
        Some(&mut client),
        Some(&url),
        Some(&settings),
        None,
        Some(&mut request_context),
    );
    if created != 1 {
        if let Ok(mut placements) = layout().lock() {
            placements.remove(&spawn.tab);
        }
        return Err("cef browser_host_create_browser failed".to_string());
    }
    Ok(())
}

/// Move/resize a browser's native surface inside its parent. Physical
/// pixels, relative to the parent. No-op when the browser is gone.
pub fn set_bounds(tab: u64, x: i32, y: i32, width: i32, height: i32) {
    if let Ok(mut placements) = layout().lock() {
        if let Some(placement) = placements.get_mut(&tab) {
            placement.y = y;
        }
    }
    let Some(child) = browsers().lock().ok().and_then(|m| m.get(&tab).map(|e| e.child))
    else {
        return;
    };
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
        };
        SetWindowPos(
            child as HWND,
            std::ptr::null_mut(),
            x,
            y,
            width.max(1),
            height.max(1),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (child, x, y, width, height);
    }
}

/// Reparent a browser's native surface into a different window — the
/// lossless half of detach/dock, matching what `Webview::reparent` does
/// for our own webviews. The document, JS heap and network streams
/// never notice.
///
/// Deliberately NOT paired with the reconciler's close-and-recreate
/// self-heal: recreating a browser would destroy the session the whole
/// feature exists to keep.
pub fn reparent(tab: u64, parent: isize) -> Result<(), String> {
    let Some(child) = browsers().lock().ok().and_then(|m| m.get(&tab).map(|e| e.child))
    else {
        return Err("no browser for this tab".to_string());
    };
    if let Ok(mut placements) = layout().lock() {
        if let Some(placement) = placements.get_mut(&tab) {
            placement.parent = parent;
        }
    }
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;
        if SetParent(child as HWND, parent as HWND).is_null() {
            return Err("SetParent failed".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (child, parent);
    }
    Ok(())
}

/// Re-x/width/height every browser parented to `parent`, keeping each
/// one's `y` — the window-resize path.
///
/// Position is deliberately preserved rather than recomputed: a
/// browser's y is either the content-rect top or the parking offset,
/// both constants, so a resize never has to know which tab is active.
/// That is the same argument [`crate::shell::layout_window`] makes for
/// our own webviews, just made explicit because `SetWindowPos` has no
/// size-only form.
pub fn relayout(parent: isize, x: i32, width: i32, height: i32) {
    let targets: Vec<(u64, i32)> = match layout().lock() {
        Ok(placements) => placements
            .iter()
            .filter(|(_, placement)| placement.parent == parent)
            .map(|(&tab, placement)| (tab, placement.y))
            .collect(),
        Err(_) => return,
    };
    for (tab, y) in targets {
        set_bounds(tab, x, y, width, height);
    }
}

/// Put a browser's surface at the TOP of its parent's child z-order.
///
/// Needed because a shell window's chrome webview spans the whole
/// window, content webviews included: whichever child was added last
/// wins the overlap. A browser created before the tab beside it, or
/// re-homed by a dock, can end up underneath — invisible, with the
/// chrome's own background painted over it. Every activation re-asserts
/// it, which is cheap and idempotent.
pub fn raise(tab: u64) {
    let Some(child) = browsers().lock().ok().and_then(|m| m.get(&tab).map(|e| e.child))
    else {
        return;
    };
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        SetWindowPos(
            child as HWND,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = child;
    }
}

/// Every browser currently parented to `parent`. Synchronous and
/// model-free, so a window's `CloseRequested` — which must decide
/// whether to defer the close before it returns — can ask.
pub fn browsers_in(parent: isize) -> Vec<u64> {
    match layout().lock() {
        Ok(placements) => placements
            .iter()
            .filter(|(_, placement)| placement.parent == parent)
            .map(|(&tab, _)| tab)
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Give the browser keyboard focus (tab select).
pub fn focus(tab: u64) {
    let Some(host) = browser(tab).and_then(|b| b.host()) else {
        return;
    };
    host.set_focus(1);
}

/// Apply a zoom level (the same units `ui_set` uses for our own
/// webviews: 0 is 100%, each ±1 is a ~20% step).
pub fn set_zoom(tab: u64, level: f64) {
    let Some(host) = browser(tab).and_then(|b| b.host()) else {
        return;
    };
    host.set_zoom_level(level);
}

/// Navigate the browser's main frame.
pub fn navigate(tab: u64, url: &str) {
    let Some(frame) = browser(tab).and_then(|b| b.main_frame()) else {
        return;
    };
    frame.load_url(Some(&CefString::from(url)));
}

/// Close a browser and wait until CEF says it is fully gone.
///
/// Waiting is not optional. The close is asynchronous and its first
/// phase deliberately CANCELS itself to flush cookies (see the module
/// docs), so a caller that tore down the parent window immediately
/// would destroy the surface mid-flush. The wait is bounded because a
/// wedged renderer must not be able to hang a tab close forever; a
/// timeout means we proceed with the teardown regardless, losing at
/// worst the unflushed tail of that session.
///
/// A tab with no browser returns immediately — every teardown path
/// calls this, including the ones for component tabs.
pub async fn close(tab: u64, timeout: std::time::Duration) {
    if !has_browser(tab) {
        return;
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Ok(mut waiters) = close_waiters().lock() {
        waiters.entry(tab).or_default().push(tx);
    }
    // Re-check under no lock but after registering: if the browser
    // vanished in between, `on_before_close` already ran and our
    // sender will never fire.
    if !has_browser(tab) {
        if let Ok(mut waiters) = close_waiters().lock() {
            waiters.remove(&tab);
        }
        return;
    }
    // Force (1): a page's `beforeunload` dialog must not be able to
    // block a tab close. `do_close` — and therefore the cookie flush —
    // still runs.
    if let Some(host) = browser(tab).and_then(|b| b.host()) {
        host.close_browser(1);
    }
    let _ = tokio::time::timeout(timeout, rx).await;
}

// ---------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------

// `FlushDone` is the cookie flush's completion leg: it flips the
// browser's flushed flag and RE-ISSUES the close that `do_close`
// cancelled, which then re-enters with the flag set and is allowed
// through. (Plain comments, not doc comments — the wrap macros match
// the struct head literally and a `#[doc]` does not parse there.)
wrap_completion_callback! {
    struct FlushDone {
        tab: u64,
        flushed: Arc<AtomicBool>,
    }

    impl CompletionCallback {
        fn on_complete(&self) {
            self.flushed.store(true, Ordering::SeqCst);
            if let Some(host) = browser(self.tab).and_then(|b| b.host()) {
                host.close_browser(1);
            }
        }
    }
}

wrap_life_span_handler! {
    struct LifeSpan {
        tab: u64,
        flushed: Arc<AtomicBool>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(b) = browser else { return };
            let child = b
                .host()
                .map(|h| raw_from_handle(h.window_handle()))
                .unwrap_or(0);
            // Clone bumps the CEF refcount so the handle outlives this
            // borrow. The flushed flag is NOT mirrored here — the two
            // handlers that drive the close each hold the same `Arc`,
            // and a third copy could only ever go stale.
            let entry = Entry {
                browser: b.clone(),
                child,
            };
            if let Ok(mut map) = browsers().lock() {
                map.insert(self.tab, entry);
            }
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            // Second pass — the flush finished, let the close through.
            if self.flushed.load(Ordering::SeqCst) {
                return 0;
            }
            // The cookie manager must come from THIS browser's request
            // context. `cookie_manager_get_global_manager` looks like
            // the shortcut and is a trap: it returns the DEFAULT
            // context's store, a different file from the one this
            // browser writes.
            let manager = browser
                .as_ref()
                .and_then(|b| b.host())
                .and_then(|h| h.request_context())
                .and_then(|rc| rc.cookie_manager(None));
            let Some(manager) = manager else {
                // Nothing to flush — don't stall the close.
                self.flushed.store(true, Ordering::SeqCst);
                return 0;
            };
            let mut cb = FlushDone::new(self.tab, self.flushed.clone());
            if manager.flush_store(Some(&mut cb)) == 1 {
                1 // accepted — `on_complete` re-issues the close
            } else {
                self.flushed.store(true, Ordering::SeqCst);
                0
            }
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            if let Ok(mut map) = browsers().lock() {
                map.remove(&self.tab);
            }
            if let Ok(mut placements) = layout().lock() {
                placements.remove(&self.tab);
            }
            if let Ok(mut waiters) = close_waiters().lock() {
                for tx in waiters.remove(&self.tab).unwrap_or_default() {
                    let _ = tx.send(());
                }
            }
        }
    }
}

// `InjectScript` injects the tab's declared script into every
// main-frame load.
wrap_load_handler! {
    struct InjectScript {
        script: Option<String>,
    }

    impl LoadHandler {
        fn on_load_start(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _transition_type: TransitionType,
        ) {
            let Some(script) = self.script.as_ref() else { return };
            let Some(frame) = frame else { return };
            // Top-level frame only — a page's iframes are not the
            // plugin's surface and must not run its code.
            if frame.is_main() != 1 {
                return;
            }
            let code = CefString::from(script.as_str());
            // Cosmetic: the name DevTools shows in stack traces.
            let script_url = CefString::from("objectiveai://tab-script.js");
            frame.execute_java_script(Some(&code), Some(&script_url), 0);
        }
    }
}

wrap_client! {
    pub struct ViewerClient {
        tab: u64,
        flushed: Arc<AtomicBool>,
        script: Option<String>,
    }

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(LifeSpan::new(self.tab, self.flushed.clone()))
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(InjectScript::new(self.script.clone()))
        }
    }
}

wrap_app! {
    pub struct ViewerApp {}

    impl App {}
}
