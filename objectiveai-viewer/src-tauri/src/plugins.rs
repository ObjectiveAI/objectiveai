//! Plugin glue: dynamic axum route registration, Tauri commands the
//! React shell calls to discover installed plugins, and the custom
//! `plugin://` URI scheme handler that serves plugin UI bundles out
//! of `<plugins_dir>/<name>/viewer/`.
//!
//! Plugin discovery goes through the cli binary: [`list_all_plugins`]
//! drives the SDK's typed `plugins list` leaf over a
//! [`BinaryExecutor`], the same executor `cli_run` uses.

use axum::Json;
use axum::http::StatusCode;
use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::plugins::list as plugins_list;
use objectiveai_sdk::cli::command::plugins::list::{
    ResponseHttpMethod, ResponseItem as PluginManifest, ResponseViewerRoute,
};

use objectiveai_sdk::viewer::{Event, EventSender};

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
pub(crate) async fn list_all_plugins(executor: &BinaryExecutor) -> Vec<PluginManifest> {
    let request = plugins_list::Request {
        path_type: plugins_list::Path::PluginsList,
        offset: None,
        limit: None,
        jq: None,
    };
    let agent_arguments = crate::cli_command::viewer_agent_arguments();
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

/// Register one plugin viewer route on the given axum router. The
/// route lands at `/plugin/<plugin>/<route.path>`; a hit emits an
/// `Event::Plugin` carrying the manifest-declared `type` tag and the
/// request body. Body-less requests (GET, or POST with no body) yield
/// `Value::Null` as the request value.
pub(crate) fn register_plugin_route(
    app: axum::Router,
    tx: EventSender,
    plugin: String,
    route: ResponseViewerRoute,
) -> axum::Router {
    let full_path = format!("/plugin/{plugin}{}", route.path);
    let r#type = route.r#type.clone();
    let method = route.method;
    let plugin_for_handler = plugin.clone();

    let handler = move |body: Option<Json<serde_json::Value>>| {
        let tx = tx.clone();
        let plugin = plugin_for_handler.clone();
        let sub_type = r#type.clone();
        async move {
            let value = body.map(|Json(v)| v).unwrap_or(serde_json::Value::Null);
            let _ = tx.send(Event::Inbound {
                destination: plugin,
                sub_type,
                value,
            });
            StatusCode::OK
        }
    };

    let method_router = match method {
        ResponseHttpMethod::Get => axum::routing::get(handler),
        ResponseHttpMethod::Post => axum::routing::post(handler),
        ResponseHttpMethod::Put => axum::routing::put(handler),
        ResponseHttpMethod::Patch => axum::routing::patch(handler),
        ResponseHttpMethod::Delete => axum::routing::delete(handler),
    };
    app.route(&full_path, method_router)
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

/// One installed plugin's viewer info, surfaced to the React shell.
/// Pre-resolves `iframe_src` on the Rust side so the frontend doesn't
/// have to know about `viewer_zip` vs `viewer_url` — it just renders
/// the URL.
#[derive(serde::Serialize, Clone, Debug)]
pub(crate) struct ViewerPluginInfo {
    /// Plugin name (== repository name == tab label).
    pub name: String,
    /// Iframe `src=` to load. For `viewer_zip` plugins this is the
    /// `plugin://localhost/<name>/index.html` URL served by
    /// [`serve_plugin_asset`]; for `viewer_url` plugins this is the
    /// manifest's URL verbatim.
    pub iframe_src: String,
    /// Manifest's `mobile_ready` flag, forwarded for future responsive
    /// layout decisions on the frontend.
    pub mobile_ready: bool,
}

/// Installed plugins that should appear as tabs in the viewer shell.
/// Pre-filtered to plugins that ship a viewer (either `viewer_zip` or
/// `viewer_url`) — without a viewer source there's nothing for the
/// iframe to render. Plugins with only `viewer_routes` and no
/// viewer source still have their axum routes registered at startup
/// but don't get a tab.
#[tauri::command]
pub(crate) async fn list_plugins_with_viewer(
    state: tauri::State<'_, BinaryExecutor>,
) -> Result<Vec<ViewerPluginInfo>, String> {
    let plugins = list_all_plugins(state.inner()).await;
    Ok(plugins
        .into_iter()
        .filter(|p| p.viewer_zip.is_some() || p.viewer_url.is_some())
        .map(|p| {
            let iframe_src = match p.viewer_url.as_deref() {
                Some(url) => url.to_string(),
                None => format!(
                    "plugin://localhost/{}/index.html",
                    percent_encode_plugin_name(&p.name)
                ),
            };
            ViewerPluginInfo {
                name: p.name,
                iframe_src,
                mobile_ready: p.mobile_ready,
            }
        })
        .collect())
}

/// Handler for the custom `plugin://` URI scheme. Resolves
/// `plugin://localhost/<plugin>/<path>` to a file under
/// `<plugins_dir>/<plugin>/viewer/<path>`. Rejects path components
/// containing `..` so a plugin can't read outside its own viewer
/// subtree. Falls back to `<path>` = `index.html` when the request
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
    if segments.is_empty() || segments.iter().any(|s| *s == "..") {
        return not_found();
    }
    if uri.path().ends_with('/') || segments.len() == 1 {
        segments.push("index.html");
    }
    let plugin = segments.remove(0);
    let rest: std::path::PathBuf = segments.iter().collect();
    let abs = plugins_dir.join(plugin).join("viewer").join(&rest);
    let canon_root = plugins_dir.join(plugin).join("viewer");

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

