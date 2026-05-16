//! Plugin glue: dynamic axum route registration, Tauri commands the
//! React shell calls to discover installed plugins, and the custom
//! `plugin://` URI scheme handler that serves plugin UI bundles out
//! of `<plugins_dir>/<name>/viewer/`.

use axum::Json;
use axum::http::StatusCode;
use objectiveai_sdk::filesystem::Client as FsClient;
use objectiveai_sdk::filesystem::plugins::{HttpMethod, ViewerRoute};

use objectiveai_sdk::viewer::{Event, EventSender};

/// Register one plugin viewer route on the given axum router. The
/// route lands at `/plugin/<plugin>/<route.path>`; a hit emits an
/// `Event::Plugin` carrying the manifest-declared `type` tag and the
/// request body. Body-less requests (GET, or POST with no body) yield
/// `Value::Null` as the request value.
pub(crate) fn register_plugin_route(
    app: axum::Router,
    tx: EventSender,
    plugin: String,
    route: ViewerRoute,
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
        HttpMethod::Get => axum::routing::get(handler),
        HttpMethod::Post => axum::routing::post(handler),
        HttpMethod::Put => axum::routing::put(handler),
        HttpMethod::Patch => axum::routing::patch(handler),
        HttpMethod::Delete => axum::routing::delete(handler),
    };
    app.route(&full_path, method_router)
}

/// Names of installed plugins that should appear as tabs in the
/// viewer shell. Pre-filtered to plugins that ship a viewer bundle
/// (`viewer_zip` set) — without a bundle there's nothing for the
/// iframe to render. Plugins with only `viewer_routes` and no
/// bundle still have their axum routes registered at startup but
/// don't get a tab.
#[tauri::command]
pub(crate) async fn list_plugins_with_viewer(
    state: tauri::State<'_, FsClient>,
) -> Result<Vec<String>, String> {
    let plugins = state.inner().list_plugins(0, usize::MAX).await;
    Ok(plugins
        .into_iter()
        .filter(|p| p.manifest.viewer_zip.is_some())
        .map(|p| p.name)
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

