//! The `plugin://` asset protocol — how a content webview reaches an
//! installed plugin's files. Path shape mirrors the identity:
//! `plugin://localhost/{owner}/{name}/{version}/{asset...}` (Windows
//! surfaces it as `http://plugin.localhost/...`) resolves to
//! `<plugins_root>/{owner}/{name}/{version}/<manifest.viewer>/{asset...}`.
//! The manifest's `viewer` sub-root is read per request — two small
//! local reads beat a cache's invalidation story. 404 on any miss.

use tauri::Manager;

/// Segments arrive from a URL-path split, so `/` never appears;
/// reject separators, dot-dirs, and percent-escapes outright (the
/// install tree is plain ASCII — an escape is an attack, not a name).
fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains('\\')
        && !segment.contains('%')
        && segment != "."
        && segment != ".."
}

fn response(
    status: u16,
    mime: Option<&str>,
    no_store: bool,
    body: Vec<u8>,
) -> tauri::http::Response<Vec<u8>> {
    let mut builder = tauri::http::Response::builder()
        .status(status)
        // A module `import()` from the app origin is a CORS-mode GET
        // (no preflight) — the ACAO header is load-bearing, on every
        // response including the 404s.
        .header("Access-Control-Allow-Origin", "*");
    if let Some(mime) = mime {
        builder = builder.header("Content-Type", mime);
    }
    if no_store {
        // Development serving: the file changes under the URL, and any
        // HTTP-layer caching would quietly defeat the hot-reload loop.
        // (The ES module map still memoizes per URL — that is what the
        // `?v=` cache-busting on the JS side is for.)
        builder = builder.header("Cache-Control", "no-store");
    }
    builder.body(body).expect("static response headers")
}

/// Resolve + read one plugin asset. `None` = 404: unknown plugin,
/// no viewer extension, invalid path shape, or missing file alike.
///
/// `tab` is the requesting content webview's tab id, when the request
/// came from one — dev-mode attribution ("which tabs consumed this
/// file") records against it.
async fn serve(
    app: &tauri::AppHandle,
    path: &str,
    tab: Option<u64>,
) -> Option<tauri::http::Response<Vec<u8>>> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // owner / name / version / at least one asset segment.
    if segments.len() < 4 || !segments.iter().all(|s| safe_segment(s)) {
        return None;
    }
    let (owner, name, version) = (segments[0], segments[1], segments[2]);

    // DEVELOPMENT override: a registered trio serves LIVE from the
    // author's directory — the manifest read fresh from its root, the
    // assets from `viewer.development.output` — and never touches the
    // install tree. An empty registry (every non-development viewer)
    // makes this a single map miss.
    let dev = app.state::<super::DevPlugins>();
    if let Some(root) = dev.get(owner, name, version) {
        let asset_root = super::dev::dev_asset_root(&root).await?;
        let mut file = asset_root;
        for segment in &segments[3..] {
            file = file.join(segment);
        }
        let bytes = tokio::fs::read(&file).await.ok()?;
        if let Some(tab) = tab {
            dev.record_consumed(file.clone(), tab);
        }
        let mime = mime_guess::from_path(&file).first_or_octet_stream();
        return Some(response(200, Some(mime.essence_str()), true, bytes));
    }

    let dirs = app.state::<super::PluginsDirs>();
    let manifest =
        super::plugins::read_manifest(&dirs.plugins_root(), owner, name, version)
            .await?;
    // A plugin with no viewer half serves nothing.
    manifest.viewer.as_ref()?;
    // The sub-root asset paths are relative to is FIXED — the
    // laboratory stages the build's output into it — so this joins a
    // constant rather than manifest data, and the two sides cannot
    // drift into 404ing everything.
    let mut file = dirs
        .plugins_root()
        .join(owner.to_lowercase())
        .join(name.to_lowercase())
        .join(version)
        .join(objectiveai_sdk::cli::plugins::VIEWER_DIR);
    for segment in &segments[3..] {
        file = file.join(segment);
    }
    let bytes = tokio::fs::read(&file).await.ok()?;
    let mime = mime_guess::from_path(&file).first_or_octet_stream();
    Some(response(200, Some(mime.essence_str()), false, bytes))
}

/// The Builder-registered handler — asynchronous so the disk reads
/// never block the platform's protocol thread (the MAIN thread on
/// Windows).
pub fn handle_plugin_protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    // The requesting webview — a `tab-<id>` label for content
    // webviews, which is the only shape attribution cares about.
    let tab = super::native::tab_id(ctx.webview_label());
    let path = request.uri().path().to_string();
    tauri::async_runtime::spawn(async move {
        let resp = serve(&app, &path, tab)
            .await
            .unwrap_or_else(|| response(404, None, false, Vec::new()));
        responder.respond(resp);
    });
}
