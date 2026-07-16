//! Plugin glue: Tauri commands the React shell calls to discover
//! installed plugins, and the custom `plugin://` URI scheme handler
//! that serves plugin UI bundles out of `<plugins_dir>/<name>/viewer/`.
//!
//! Plugin discovery goes through the daemon: [`list_all_plugins`]
//! drives the SDK's typed `plugins list` leaf over the
//! [`SseCommandExecutor`]. Plugin iframes currently have NO daemon
//! access — the webview no longer holds daemon coordinates (every
//! daemon stream rides [`crate::daemon_proxy`]'s Tauri commands,
//! which a sandboxed iframe cannot invoke); see the TODO(plugins) in
//! `src/PluginPane.tsx`.

use futures::StreamExt;
use objectiveai_sdk::cli::command::sse::SseCommandExecutor;
use objectiveai_sdk::cli::command::plugins::list as plugins_list;
use objectiveai_sdk::cli::command::plugins::list::ResponseItem as PluginManifest;

/// Per-call identity for shell-initiated cli runs: instance
/// hierarchy `"Viewer"`, every other field `None` so the daemon
/// clears rather than inherits it — nothing leaks from the daemon's
/// own environment into a viewer-initiated run.
pub(crate) fn viewer_agent_arguments() -> objectiveai_sdk::cli::command::AgentArguments {
    objectiveai_sdk::cli::command::AgentArguments {
        agent_instance_hierarchy: Some("Viewer".to_string()),
        ..Default::default()
    }
}

/// `<objectiveai_dir>/bin/plugins` — the root the `plugin://` URI
/// scheme serves assets from (plugins are machine-wide, shared by
/// every state). The dir arrives already resolved by
/// `ConfigBuilder::build`, so assets and the spawned cli always
/// agree on the install root.
pub(crate) fn plugins_dir(objectiveai_dir: &std::path::Path) -> std::path::PathBuf {
    objectiveai_dir.join("bin").join("plugins")
}

/// List every installed plugin manifest by spawning
/// `objectiveai plugins list` through the executor. Resilient on
/// purpose: a missing binary or a malformed line is logged to stderr
/// and yields an empty/partial list rather than failing viewer
/// startup — a viewer with zero plugin tabs is still a working viewer.
pub(crate) async fn list_all_plugins(executor: &SseCommandExecutor) -> Vec<PluginManifest> {
    let request = plugins_list::Request {
        path_type: plugins_list::Path::PluginsList,
        offset: None,
        limit: None,
        base: Default::default(),
    };
    let agent_arguments = viewer_agent_arguments();
    let mut stream = match plugins_list::execute(executor, request, Some(&agent_arguments)).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("plugins list failed to spawn the cli: {e:?}");
            return Vec::new();
        }
    };
    let mut plugins = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(manifest) => plugins.push(manifest),
            Err(e) => eprintln!("plugins list emitted an error item: {e:?}"),
        }
    }
    plugins
}

/// Percent-encode characters in a plugin name that would change the
/// meaning of a URL path segment. GitHub repo names are constrained
/// to `[A-Za-z0-9._-]+` which need no encoding, but defend in depth
/// against future name conventions or hand-edited manifests.
/// Dependency-free; no need for the full `percent-encoding` crate.
fn percent_encode_plugin_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Managed-state wrapper for the plugins root, so Tauri commands (not
/// just the `plugin://` protocol handler) can resolve on-disk plugin
/// bundles at `<plugins_dir>/<owner>/<name>/<version>/viewer/`.
pub(crate) struct PluginsDir(pub(crate) std::path::PathBuf);

/// One installed plugin's viewer info, surfaced to the React shell.
/// Pre-resolves `iframe_src` on the Rust side so the frontend doesn't
/// have to know about a remote `viewer_url` vs an on-disk bundle — it
/// just renders the URL.
#[derive(serde::Serialize, Clone, Debug)]
pub(crate) struct ViewerPluginInfo {
    /// Plugin owner (GitHub `<owner>` segment).
    pub owner: String,
    /// Plugin name (== repository name == tab label).
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Iframe `src=` to load. For on-disk-bundle plugins this is the
    /// `plugin://localhost/<owner>/<name>/<version>/index.html` URL
    /// served by [`serve_plugin_asset`]; for `viewer_url` plugins this
    /// is the manifest's URL verbatim.
    pub iframe_src: String,
}

/// Installed plugins that should appear as tabs in the viewer shell.
/// Pre-filtered to plugins that ship a viewer: either a remote
/// `viewer_url`, or an extracted on-disk bundle (an `index.html` under
/// `<plugins_dir>/<owner>/<name>/<version>/viewer/`). The get response
/// no longer carries `viewer_zip`, so bundle presence is read from
/// disk. Plugins without a viewer source don't get a tab.
#[tauri::command]
pub(crate) async fn list_plugins_with_viewer(
    executor: tauri::State<'_, SseCommandExecutor>,
    plugins_dir: tauri::State<'_, PluginsDir>,
) -> Result<Vec<ViewerPluginInfo>, String> {
    let plugins = list_all_plugins(executor.inner()).await;
    let root = &plugins_dir.inner().0;
    Ok(plugins
        .into_iter()
        .filter_map(|p| {
            // Remote viewer URL wins; otherwise look for an extracted
            // bundle on disk at the plugin's version folder.
            if let Some(url) = p.viewer_url.as_deref() {
                return Some(ViewerPluginInfo {
                    owner: p.owner,
                    name: p.name,
                    version: p.version,
                    iframe_src: url.to_string(),
                });
            }
            let bundle_index = root
                .join(&p.owner)
                .join(&p.name)
                .join(&p.version)
                .join("viewer")
                .join("index.html");
            if !bundle_index.exists() {
                return None;
            }
            let iframe_src = format!(
                "plugin://localhost/{}/{}/{}/index.html",
                percent_encode_plugin_name(&p.owner),
                percent_encode_plugin_name(&p.name),
                percent_encode_plugin_name(&p.version),
            );
            Some(ViewerPluginInfo {
                owner: p.owner,
                name: p.name,
                version: p.version,
                iframe_src,
            })
        })
        .collect())
}

/// Handler for the custom `plugin://` URI scheme. Resolves
/// `plugin://localhost/<owner>/<name>/<version>/<path>` to a file under
/// `<plugins_dir>/<owner>/<name>/<version>/viewer/<path>` (the nested
/// install layout). Rejects path components containing `..` so a plugin
/// can't read outside its own viewer subtree. Falls back to
/// `<path>` = `index.html` when no file part is given or the request
/// path ends with `/`.
pub(crate) fn serve_plugin_asset(
    plugins_dir: &std::path::Path,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let uri = request.uri();
    let mut segments: Vec<&str> = uri
        .path()
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    // Require the <owner>/<name>/<version> coordinate prefix and reject
    // traversal.
    if segments.len() < 3 || segments.iter().any(|s| *s == "..") {
        return not_found();
    }
    let owner = segments.remove(0);
    let name = segments.remove(0);
    let version = segments.remove(0);
    if segments.is_empty() || uri.path().ends_with('/') {
        segments.push("index.html");
    }
    let rest: std::path::PathBuf = segments.iter().collect();
    let viewer_root = plugins_dir
        .join(owner)
        .join(name)
        .join(version)
        .join("viewer");
    let abs = viewer_root.join(&rest);
    let canon_root = viewer_root;

    // Path-traversal defense: the resolved path must remain inside
    // <plugins_dir>/<plugin>/viewer/.
    let canon_abs = match abs.canonicalize() {
        Ok(p) => p,
        Err(_) => return not_found(),
    };
    let canon_root = match canon_root.canonicalize() {
        Ok(p) => p,
        Err(_) => return not_found(),
    };
    if !canon_abs.starts_with(&canon_root) {
        return not_found();
    }

    let bytes = match std::fs::read(&canon_abs) {
        Ok(b) => b,
        Err(_) => return not_found(),
    };
    let mime = guess_mime(&canon_abs);
    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", mime)
        .body(bytes)
        .unwrap_or_else(|_| not_found())
}

fn not_found() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(404)
        .body(b"not found".to_vec())
        .unwrap()
}

fn guess_mime(path: &std::path::Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string()
}

