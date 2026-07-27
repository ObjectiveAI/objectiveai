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

use super::model::{ROOT_IDENTITY, ShellModel, Snapshot, TabKind, UiState};
use super::native;

/// One tab open request — everything a tab is, supplied by the
/// opener. The sender's IDENTITY is never part of it: Rust derives
/// it from the calling webview and `module` resolves against that
/// identity's root. The TS mirror is the SDK's `ViewerOpenTab`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenTab {
    /// Component module path, identity-root-relative. Required for a
    /// component tab; omitted (with `url` supplied) for a browser.
    #[serde(default)]
    pub module: Option<String>,
    /// The export holding the component (`None` = `"default"`).
    #[serde(default)]
    pub export: Option<String>,
    /// The tab's display title.
    pub title: String,
    /// Opaque component props, stored verbatim.
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    /// Whether the strip shows a close button (`None` = closable).
    /// The chrome seeds the permanent home tabs with `false`.
    #[serde(default)]
    pub closable: Option<bool>,
    /// OPTIONAL identity icon path, identity-root-relative (same
    /// validation as `module`). Cosmetic, like `title` — not part of
    /// the dedupe kind.
    #[serde(default)]
    pub icon: Option<String>,
    /// OPTIONAL stylesheets, identity-root-relative (same validation
    /// as `module`). Injected and AWAITED before the component
    /// renders — cosmetic, like `icon`, and likewise not part of the
    /// dedupe kind.
    #[serde(default)]
    pub styles: Option<Vec<String>>,
    /// OPTIONAL name for the spawned tab, unique among THIS caller's
    /// children — the address it is messaged at afterwards (see
    /// [`super::TabMail`]). Unlike the cosmetics above it IS part of
    /// the dedupe kind: two children of one component under different
    /// keys are different tabs.
    #[serde(default)]
    pub key: Option<String>,
    /// Supplying this makes the tab a BROWSER: a real Chromium
    /// surface opened here, with no module and no bootstrap.
    /// Mutually exclusive with `module`.
    #[serde(default)]
    pub url: Option<String>,
    /// Browser only — the name of one of the owning plugin's manifest
    /// `scripts`, injected into every main-frame load.
    #[serde(default)]
    pub script: Option<String>,
    /// Browser only — the profile key. Present ⇒ cookies and storage
    /// PERSIST on disk and reload next time; absent ⇒ in-memory only.
    #[serde(default)]
    pub state: Option<String>,
}

/// What a content webview learns about itself at boot — everything
/// the ONE generic bootstrap needs to render: the module coordinates
/// to `import()`, the props, the labels. No JS-side resolver exists.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TabDescriptor {
    pub identity: String,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<String>,
    /// See [`TabKind::root_module`] — the bootstrap skips plugin
    /// origin prefixing when set.
    #[serde(rename = "rootModule", skip_serializing_if = "std::ops::Not::not")]
    pub root_module: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    /// Stylesheets to inject and AWAIT before rendering — the
    /// bootstrap's whole reason to know about them. Empty for every
    /// tab that declares none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<String>,
    pub title: String,
}

/// Whose namespace does this webview speak? Chrome speaks the root
/// identity; a content webview speaks whatever identity OWNS its tab
/// — a fact read from the model, never from the caller.
pub(crate) async fn sender_identity(webview: &tauri::Webview, model: &ShellModel) -> String {
    match native::tab_id(webview.label()) {
        None => ROOT_IDENTITY.to_string(),
        Some(id) => match model.tab(id).await {
            Some(tab) => tab.kind.identity,
            None => ROOT_IDENTITY.to_string(),
        },
    }
}

/// `module` must stay INSIDE the sender identity's root: a plain
/// absolute-from-root path, no scheme, no traversal. This check is
/// the load-bearing seam that makes caller-supplied modules safe.
pub(crate) fn validate_module(module: &str) -> Result<(), String> {
    if module.starts_with('/')
        && !module.contains("://")
        && !module.contains('\\')
        && !module.starts_with("//")
        && module.split('/').all(|segment| segment != "..")
    {
        Ok(())
    } else {
        Err(format!("invalid module path {module:?}"))
    }
}

/// The model snapshot — a chrome's boot read. (Subscribe to
/// `tabs://changed` FIRST, then snapshot; apply either only when the
/// generation advances.)
#[tauri::command]
pub async fn tabs_snapshot(
    model: tauri::State<'_, ShellModel>,
) -> Result<Snapshot, String> {
    Ok(model.snapshot().await)
}

/// Open a tab: if one with this exact kind (identity + module +
/// export + arguments) exists ANYWHERE, activate + focus its window
/// (open-or-focus); otherwise append a fresh tab to the CALLER's
/// window and activate it. The sender's identity is baked in — a
/// caller can only ever open tabs whose code lives under its own
/// root.
#[tauri::command]
pub async fn tabs_open(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    tab: OpenTab,
) -> Result<(), String> {
    // Exactly one surface. `validate_module` governs component paths
    // only — a browser URL is not identity-root-relative.
    let surface = match (tab.module, &tab.url) {
        (Some(_), Some(_)) => {
            return Err(
                "tabs_open: `module` and `url` are mutually exclusive — a tab is a component or a browser"
                    .to_string(),
            );
        }
        (None, None) => {
            return Err("tabs_open: one of `module` or `url` is required".to_string());
        }
        (Some(module), None) => {
            validate_module(&module)?;
            super::Surface::Component {
                module,
                export: tab.export,
                root_module: false,
            }
        }
        (None, Some(_)) => super::Surface::Browser {
            url: tab.url.clone().expect("checked Some"),
            script: tab.script.clone(),
            state: tab.state.clone(),
        },
    };
    if let Some(icon) = &tab.icon {
        validate_module(icon)?;
    }
    for style in tab.styles.iter().flatten() {
        validate_module(style)?;
    }
    // A keyed open addresses a mailbox, and a mailbox is keyed by the
    // CALLER's tab — so chrome (which has no tab) cannot open one.
    let parent = native::tab_id(webview.label());
    if tab.key.is_some() && parent.is_none() {
        return Err(
            "tabs_open: `key` requires a content webview — chrome spawns no children"
                .to_string(),
        );
    }
    let identity = sender_identity(&webview, &model).await;
    let kind = TabKind {
        identity,
        key: tab.key.clone(),
        arguments: tab.arguments,
        surface,
    };
    let caller = webview.window().label().to_string();
    let opened = open_tab(
        &app,
        &caller,
        kind,
        tab.title,
        tab.closable.unwrap_or(true),
        tab.icon,
        tab.styles.unwrap_or_default(),
        true,
    )
    .await;
    // Bind AFTER the open, using the id it actually landed on: a
    // dedupe hit reuses an existing tab, and rebinding must rejoin
    // that mailbox rather than reset it.
    if let (Some(parent), Some(key)) = (parent, tab.key) {
        app.state::<super::TabMail>()
            .bind(parent, key, opened.tab_id)
            .await;
    }
    if let Some(label) = opened.focus {
        if let Some(target) = app.get_window(&label) {
            let _ = target.set_focus();
        }
    }
    Ok(())
}

/// The one internal open path — the command above, the boot
/// orchestrator, and the toggle all land here: mutate the model,
/// publish, reconcile. `activate` = whether the tab becomes active
/// (a toggle-on appends QUIETLY); focus is the caller's decision.
pub(crate) async fn open_tab(
    app: &tauri::AppHandle,
    window: &str,
    kind: TabKind,
    title: String,
    closable: bool,
    icon: Option<String>,
    styles: Vec<String>,
    activate: bool,
) -> super::model::Opened {
    let model = app.state::<ShellModel>();
    let opened = model
        .open_or_focus(
            window,
            kind,
            title,
            closable,
            icon,
            styles,
            activate,
            |label| app.get_window(label).is_some(),
        )
        .await;
    native::publish(app, &opened.snapshot, &opened.touched);
    // Reconcile in the BACKGROUND (sync serializes on its own mutex
    // and always takes a fresh snapshot): the open returns at
    // publish, so a BURST of opens — the home-tab seeding, the
    // plugin loader — lands in the strip at once instead of gating
    // each tab behind the previous webview's creation.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        native::sync(&app).await;
    });
    opened
}

/// "What am I": the calling content webview's OWN descriptor,
/// resolved from its label — the generic bootstrap's one boot read.
/// Self-scoped by construction: a content webview can learn only
/// about itself, never the registry.
#[tauri::command]
pub async fn tab_self(
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
) -> Result<TabDescriptor, String> {
    let id = native::tab_id(webview.label())
        .ok_or_else(|| "tab_self: not a content webview".to_string())?;
    let tab = model
        .tab(id)
        .await
        .ok_or_else(|| "tab_self: unknown tab".to_string())?;
    // Only a component tab has a descriptor to fetch — a browser has
    // no bootstrap to ask, so reaching here means a stale caller.
    let super::Surface::Component {
        module,
        export,
        root_module,
    } = tab.kind.surface
    else {
        return Err("tab_self: this tab is a browser, not a component".to_string());
    };
    Ok(TabDescriptor {
        identity: tab.kind.identity,
        module,
        export,
        root_module,
        arguments: tab.kind.arguments,
        styles: tab.styles,
        title: tab.title,
    })
}

/// Activate a tab in the calling window, and hand it keyboard focus.
/// Unknown ids no-op.
#[tauri::command]
pub async fn tabs_select(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    tab_id: u64,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();
    select_tab(&app, &caller, tab_id).await;
    Ok(())
}

/// The internal select path — the command above and the boot
/// orchestrator's select-first both land here. Focus follows an
/// EXPLICIT selection (the reconciler itself never focuses — an
/// unrelated mutation must not steal it).
pub(crate) async fn select_tab(app: &tauri::AppHandle, window: &str, tab_id: u64) {
    let model = app.state::<ShellModel>();
    let Some(snapshot) = model.select(window, tab_id).await else {
        return;
    };
    native::publish(app, &snapshot, &[window.to_string()]);
    native::sync(app).await;
    if let Some(webview) = app.get_webview(&native::tab_label(tab_id)) {
        let _ = webview.set_focus();
    }
    // A browser tab has no webview to focus — its keyboard focus is
    // CEF's to give. No-op for every component tab.
    crate::cef::focus(tab_id);
}

/// Close a tab (idempotent). A window whose last tab closes is
/// itself closed — every window; when the LAST window goes, the app
/// exits (the Destroyed handler's empty-check).
#[tauri::command]
pub async fn tabs_close(
    app: tauri::AppHandle,
    tab_id: u64,
) -> Result<(), String> {
    close_tab(&app, tab_id).await;
    Ok(())
}

/// The internal close path — the command above, the channel-offer
/// withdrawal handler, and the self-close land here.
pub(crate) async fn close_tab(app: &tauri::AppHandle, tab_id: u64) {
    // Every removal path must reach the mailbox registry, or a peer
    // blocked on this tab waits forever.
    app.state::<super::TabMail>().closed(tab_id).await;
    // A browser tab's surface goes NOW — hidden synchronously, torn
    // down behind us. Awaiting the teardown here would hold the tab in
    // the strip for the length of a cookie flush, which reads as a
    // frozen app; and it is unnecessary, because the thing that must
    // not happen early (destroying the parent HWND) is gated by the
    // window's own close, not by this. No-op for a component tab.
    super::browser::begin_close(app, tab_id);
    let model = app.state::<ShellModel>();
    let Some(closed) = model.close(tab_id).await else {
        return;
    };
    native::publish(app, &closed.snapshot, &closed.touched);
    native::sync(app).await;
    if let Some(label) = closed.close_window {
        if let Some(window) = app.get_window(&label) {
            let _ = window.close();
        }
    }
}

/// Close the CALLING content webview's OWN tab — self-scoped like
/// `tab_self` (the descriptor carries no tab id; the label does).
/// The channel-request tab's Decline rides this; a window whose sole
/// tab this is closes with it.
#[tauri::command]
pub async fn tabs_close_self(
    app: tauri::AppHandle,
    webview: tauri::Webview,
) -> Result<(), String> {
    let id = native::tab_id(webview.label())
        .ok_or_else(|| "tabs_close_self: not a content webview".to_string())?;
    close_tab(&app, id).await;
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
    inventory: tauri::State<'_, super::TabInventory>,
    tab_id: u64,
    index: usize,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();
    let Some(snapshot) = model.move_tab(&caller, tab_id, index).await else {
        return Ok(());
    };
    // A manual strip reorder: the live strip is the USER's for the
    // rest of this session — pane reorders stop live-applying.
    inventory.set_user_controlled();
    native::publish(&app, &snapshot, &[caller]);
    Ok(())
}

/// Tear `tab_id` out of the calling window into a FRESH shell window
/// under the cursor, then hand the user's still-held drag to the OS
/// (`start_dragging`). The tab's content webview RIDES ALONG — a
/// reparent, not a rebuild: nothing in it reloads. Idempotent per tab
/// (a second racing call finds the tab already moved and no-ops).
/// Dragging a window's LAST tab drags the window itself — every
/// window, no exceptions (no window is special): detach can never
/// leave an empty window behind.
#[tauri::command]
pub async fn tabs_detach(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    inventory: tauri::State<'_, super::TabInventory>,
    tab_id: u64,
) -> Result<(), String> {
    let caller = webview.window().label().to_string();

    // A pop-out: user-controlled for the session — set BEFORE the
    // sole-tab shortcut, so even dragging the only window's sole tab
    // (a plain window drag) counts, per spec.
    inventory.set_user_controlled();

    // Sole tab: the whole window IS the tab — drag the window itself.
    if model.is_sole_tab(&caller, tab_id).await {
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

/// One declared root tab — the chrome's manifest-equivalent, sent
/// once per chrome boot via [`tabs_declare`].
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeclareEntry {
    pub name: String,
    pub title: String,
    pub module: String,
    #[serde(default)]
    pub export: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    pub closable: bool,
    #[serde(default)]
    pub permanent: bool,
}

/// The chrome declares the ROOT identity's tab inventory — how Rust
/// learns the built-in tabs without hardcoding a single name. Every
/// chrome calls this on mount; only the FIRST declaration per app
/// run applies (all chromes ship the same build, so later ones are
/// identical no-ops). Chrome-only: content webviews may not declare.
#[tauri::command]
pub async fn tabs_declare(
    webview: tauri::Webview,
    inventory: tauri::State<'_, super::TabInventory>,
    entries: Vec<DeclareEntry>,
) -> Result<(), String> {
    let label = webview.label();
    if native::tab_id(label).is_some() || !label.starts_with("chrome-") {
        return Err("tabs_declare: chrome webviews only".to_string());
    }
    if entries.is_empty() || entries.len() > 32 {
        return Err(format!("tabs_declare: {} entries", entries.len()));
    }
    let mut seen = std::collections::HashSet::new();
    let mut roots = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.name.is_empty() || !seen.insert(entry.name.clone()) {
            return Err(format!("tabs_declare: bad/duplicate name {:?}", entry.name));
        }
        validate_module(&entry.module)?;
        if let Some(icon) = &entry.icon {
            validate_module(icon)?;
        }
        roots.push(super::TabEntry {
            identity: ROOT_IDENTITY.to_string(),
            identity_key: ROOT_IDENTITY.to_string(),
            name: entry.name,
            title: entry.title,
            module: entry.module,
            export: entry.export,
            icon: entry.icon,
            // Root tabs are bundled by vite, which emits their CSS
            // into the document itself — nothing to inject.
            styles: Vec::new(),
            // A permanent tab is never strip-closable, whatever the
            // declaration says.
            closable: entry.closable && !entry.permanent,
            permanent: entry.permanent,
        });
    }
    inventory.declare(roots).await;
    Ok(())
}

/// The full tab inventory with resolved toggles — the tabs tab's
/// read. ROOT-identity callers only (a plugin tab's webview has IPC
/// access but must not enumerate or toggle the shell).
#[tauri::command]
pub async fn tabs_inventory(
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    inventory: tauri::State<'_, super::TabInventory>,
) -> Result<Vec<super::InventoryEntry>, String> {
    if sender_identity(&webview, &model).await != ROOT_IDENTITY {
        return Err("tabs_inventory: root identity only".to_string());
    }
    Ok(inventory.inventory().await)
}

/// Config-order live-slot: reorder every window's inventory tabs to
/// `kinds` and publish. Publish only — a permutation changes neither
/// placement nor the active tab, so no reconcile (the tabs_move
/// precedent). Shared by tabs_toggle, tabs_reorder, and the plugin
/// installer's rescan.
pub(crate) async fn live_slot(
    app: &tauri::AppHandle,
    model: &ShellModel,
    kinds: &[TabKind],
) {
    if let Some(snapshot) = model.reorder_all(kinds).await {
        native::publish(app, &snapshot, &[]);
    }
}

/// Toggle one inventory tab. Enabled = intent, persisted forever
/// (missing = enabled): enabling opens the tab into the CALLING
/// window, disabling closes the live tab wherever it is (bypassing
/// strip closability — home tabs retire through here). Permanent
/// entries refuse. Emits `inventory://changed` so every open tabs
/// pane live-updates.
#[tauri::command]
pub async fn tabs_toggle(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    inventory: tauri::State<'_, super::TabInventory>,
    identity_key: String,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    if sender_identity(&webview, &model).await != ROOT_IDENTITY {
        return Err("tabs_toggle: root identity only".to_string());
    }
    let Some(entry) = inventory.entry(&identity_key, &name).await else {
        return Err(format!("tabs_toggle: unknown tab {identity_key:?}/{name:?}"));
    };
    if entry.permanent {
        return Err(format!("tabs_toggle: {identity_key:?}/{name:?} is permanent"));
    }
    // A toggle never changes display order (writes materialize the
    // order first), so the live-apply input can be computed up front
    // — keeping the persist/apply futures on ONE lock each.
    let live_apply_kinds = if enabled && !inventory.user_controlled() {
        Some(inventory.display_kinds().await)
    } else {
        None
    };
    // Persist and apply IN PARALLEL — the disk write and the model
    // mutation are independent.
    let persist = inventory.set_enabled(&identity_key, &name, enabled);
    let apply = async {
        if enabled {
            // QUIET append — the user is working in the tabs pane;
            // enabling must not yank them to the new tab.
            let window = webview.window().label().to_string();
            open_tab(
                &app,
                &window,
                entry.kind(),
                entry.title.clone(),
                entry.closable,
                entry.icon.clone(),
                entry.styles.clone(),
                false,
            )
            .await;
            // Outside user-controlled mode the strip mirrors the
            // config order — slot the reopened tab into place
            // instead of leaving it at the end.
            if let Some(kinds) = &live_apply_kinds {
                live_slot(&app, &model, kinds).await;
            }
        } else if let Some(closed) = {
            // A disabled tab may be parked in a subscribe — end the
            // wait before the tab stops existing.
            if let Some(tab_id) = model.tab_id_of(&entry.kind()).await {
                app.state::<super::TabMail>().closed(tab_id).await;
            }
            model.remove_by_kind(&entry.kind()).await
        } {
            native::publish(&app, &closed.snapshot, &closed.touched);
            native::sync(&app).await;
            if let Some(label) = closed.close_window {
                if let Some(window) = app.get_window(&label) {
                    let _ = window.close();
                }
            }
        }
    };
    let (persisted, ()) = tokio::join!(persist, apply);
    if let Err(e) = persisted {
        super::report_shell(&app, "error", format!("tabs: persist toggle: {e}")).await;
    }
    use tauri::Emitter;
    let _ = app.emit("inventory://changed", &inventory.inventory().await);
    Ok(())
}

/// One (identityKey, name) pair in a pane-sent display order.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReorderRef {
    pub identity_key: String,
    pub name: String,
}

/// Persist a new tab order from the tabs pane (the FULL display
/// order — every loaded entry, enabled and disabled). Outside
/// user-controlled mode the live strip follows; inside it, the
/// order only takes effect next boot. Root-identity callers only.
#[tauri::command]
pub async fn tabs_reorder(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, ShellModel>,
    inventory: tauri::State<'_, super::TabInventory>,
    order: Vec<ReorderRef>,
) -> Result<(), String> {
    if sender_identity(&webview, &model).await != ROOT_IDENTITY {
        return Err("tabs_reorder: root identity only".to_string());
    }
    let d: Vec<(String, String)> = order
        .into_iter()
        .map(|r| (r.identity_key, r.name))
        .collect();
    // Inventory lock first, fully released before any model work —
    // the two locks are never held together.
    match inventory.reorder(d).await {
        Err(rejected) => return Err(rejected),
        Ok(Err(io)) => {
            super::report_shell(&app, "error", format!("tabs: persist order: {io}")).await;
        }
        Ok(Ok(())) => {}
    }
    if !inventory.user_controlled() {
        live_slot(&app, &model, &inventory.display_kinds().await).await;
    }
    use tauri::Emitter;
    let _ = app.emit("inventory://changed", &inventory.inventory().await);
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
