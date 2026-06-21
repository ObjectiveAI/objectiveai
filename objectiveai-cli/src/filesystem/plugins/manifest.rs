use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::filesystem::tools::{CliZip, Exec};

/// Declarative metadata a plugin ships with — the on-disk
/// `objectiveai.json` at
/// `<base_dir>/plugins/<owner>/<name>/<version>/objectiveai.json`. The
/// CLI reads and writes this shape verbatim (install writes the fetched
/// manifest exactly as authored); the `plugins get` wire response is a
/// lean projection of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.plugins.Manifest")]
pub struct Manifest {
    /// GitHub `<owner>` segment of the source repo, written verbatim
    /// from the fetched manifest (forks land on disk with whatever
    /// owner they authored, not rewritten to the install coordinate).
    pub owner: String,

    /// The plugin's identifier (matches the `<name>` directory
    /// segment).
    pub name: String,

    /// Version string. Semver convention is recommended but not
    /// enforced — the host just displays whatever's here.
    pub version: String,

    /// One-line description of what the plugin does. Surfaced in
    /// listings and the plugin's `--help`-equivalent UI.
    pub description: String,

    /// Per-OS exec command for the plugin's cli side — the same shape
    /// tools use. The current platform's vector is the program plus
    /// its leading arguments; the run's args are appended, and the
    /// whole thing runs with CWD = `<plugin dir>/cli/`. Relative
    /// program paths resolve against that folder; bare names keep
    /// PATH-lookup semantics. An empty per-OS vector means viewer-only
    /// for that platform.
    pub exec: Exec,

    /// Per-OS cli bundle zip filenames. At install the current
    /// platform's entry (when present) is downloaded and extracted
    /// into `<plugin dir>/cli/` (the [`Self::exec`] working
    /// directory), the same way [`Self::viewer_zip`] extracts to
    /// `viewer/`. Required field; each per-OS entry is optional —
    /// absent means the exec needs nothing on disk for that platform
    /// (e.g. it invokes a PATH-resolved program, or the plugin is
    /// viewer-only).
    pub cli_zip: CliZip,

    /// GitHub-release asset filename for the plugin's viewer UI
    /// bundle (a `.zip` whose root contains `index.html` plus
    /// assets). When absent, the plugin has no viewer tab from this
    /// source. Mutually exclusive with [`Self::viewer_url`] —
    /// validated by [`Self::validate`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer_zip: Option<String>,

    /// Remote URL the viewer's iframe loads directly, instead of an
    /// on-disk bundle from [`Self::viewer_zip`]. The full URL is used
    /// as the iframe `src=` verbatim — query string, path, port,
    /// fragment all pass through. Must use `https://`, or `http://`
    /// targeting `localhost` / `127.0.0.1` (development only).
    ///
    /// Mutually exclusive with [`Self::viewer_zip`]. [`Self::viewer_routes`]
    /// apply to remote-URL viewers the same way they apply to
    /// zip-bundled viewers — the embedded axum server still hosts the
    /// declared routes; the iframe still receives the same postMessage
    /// protocol regardless of where its HTML/JS loaded from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer_url: Option<String>,

    /// HTTP routes the viewer exposes on behalf of this plugin.
    /// Each entry registers a handler at
    /// `/plugin/<repository>/<path>` on the viewer's embedded axum
    /// server; a hit emits a `PluginRequest { type, value }` event
    /// to the React frontend, which dispatches to the plugin's
    /// iframe via the postMessage bridge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewer_routes: Vec<ViewerRoute>,

    /// MCP servers the plugin wants the host to expose. Each entry
    /// has a `name` (the identifier agents reference via
    /// [`objectiveai_sdk::agent::ClientObjectiveaiMcpPluginEntry::mcp_servers`])
    /// plus an `authorization` flag. Auth-requiring entries flag
    /// `authorization = true`; credentials are resolved by the host
    /// (env vars / config), not the manifest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Vec<McpServer>,

    /// When `true`, the plugin participates in the per-state plugin
    /// daemon (`daemon spawn`): the daemon launches it as
    /// `<exec> daemon begin` (via the shared plugin executor) and keeps
    /// it resident. Host-internal — not surfaced on the `plugins get`
    /// wire projection. Skipped on serialization when `false` so
    /// non-daemon manifests stay lean.
    #[serde(default, skip_serializing_if = "is_false")]
    #[schemars(extend("omitempty" = true))]
    pub daemon: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Manifest {
    /// Whether this plugin presents a viewer tab in the host. True
    /// iff either viewer source field — [`Self::viewer_zip`] or
    /// [`Self::viewer_url`] — is set.
    pub fn has_viewer(&self) -> bool {
        self.viewer_zip.is_some() || self.viewer_url.is_some()
    }

    /// Validate fields that can't be enforced by serde alone:
    /// `viewer_zip` and `viewer_url` are mutually exclusive, and
    /// `viewer_url` (when present) must be `https://` or `http://`
    /// targeting `localhost` / `127.0.0.1`. Called at every parse
    /// boundary (remote-fetched install, on-disk read) so a broken
    /// manifest can't sneak through.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.viewer_zip.is_some() && self.viewer_url.is_some() {
            return Err("viewer_zip and viewer_url are mutually exclusive");
        }
        if let Some(url) = self.viewer_url.as_deref() {
            validate_viewer_url(url)?;
        }
        // Each MCP-server entry needs a non-empty name; the list as a
        // whole must have no `name` duplicates (since agents reference
        // by name).
        for entry in &self.mcp_servers {
            if entry.name.is_empty() {
                return Err("mcp_servers[i].name cannot be empty");
            }
        }
        for (i, a) in self.mcp_servers.iter().enumerate() {
            for b in &self.mcp_servers[i + 1..] {
                if a.name == b.name {
                    return Err("mcp_servers contains duplicate name");
                }
            }
        }
        Ok(())
    }
}

/// Allow `https://*`. Allow `http://` only when the host is
/// `localhost` or `127.0.0.1` (development). Reject everything else
/// — raw http on a public hostname inside a Tauri WebView is a
/// footgun (plaintext, MITM-able, mixed-content-blocked by the
/// browser engine in most cases).
///
/// Dependency-free: a couple of `starts_with` / split checks beat
/// pulling the full `url` crate for one validation. Doesn't handle
/// IPv6 brackets or punycode — neither matters for the localhost
/// allow-list.
fn validate_viewer_url(url: &str) -> Result<(), &'static str> {
    let url = url.trim();
    if url.is_empty() {
        return Err("viewer_url cannot be empty");
    }
    if url.starts_with("https://") {
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("http://") {
        // Host ends at the first '/', ':', '?', '#', or EOF.
        let host_end = rest
            .find(|c: char| matches!(c, '/' | ':' | '?' | '#'))
            .unwrap_or(rest.len());
        let host = &rest[..host_end];
        if host == "localhost" || host == "127.0.0.1" {
            return Ok(());
        }
        return Err(
            "viewer_url with http:// scheme is only allowed for localhost or 127.0.0.1",
        );
    }
    Err("viewer_url must use https:// or http://localhost / http://127.0.0.1")
}

/// MCP server entry inside [`Manifest::mcp_servers`]. A host-exposed
/// upstream identified by a `name` that agent declarations reference
/// (via
/// [`objectiveai_sdk::agent::ClientObjectiveaiMcpPluginEntry::mcp_servers`]),
/// plus an `authorization` flag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "filesystem.plugins.McpServer")]
pub struct McpServer {
    /// Author-chosen identifier. Unique per plugin manifest. Agents
    /// declare which subset of the plugin's MCP servers they want
    /// exposed by listing names here.
    pub name: String,
    /// Whether the host should attach an `Authorization` header
    /// when dialing this upstream. Credentials are resolved by the
    /// host (env vars / config), not the manifest.
    #[serde(default)]
    pub authorization: bool,
}

/// One HTTP route a plugin's viewer registers on the host viewer's
/// embedded axum server. The full path served is
/// `/plugin/<repository>/<self.path>`; on a hit, the body is
/// JSON-decoded and forwarded as a `PluginRequest { type: self.type,
/// value: body }` event to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.plugins.ViewerRoute")]
pub struct ViewerRoute {
    /// Path relative to the plugin's namespace. Must start with `/`;
    /// the host prepends `/plugin/<repository>` before registering.
    pub path: String,

    /// HTTP method this route handles. Methods other than the listed
    /// five aren't supported (and don't appear in plugin practice).
    pub method: HttpMethod,

    /// String tag forwarded to the plugin's iframe as the `type`
    /// field of the resulting `PluginRequest`. Plugin authors pick
    /// any value they want; the host doesn't interpret it.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// HTTP methods supported by [`ViewerRoute`]. Serializes as upper-case
/// (`"GET"`, `"POST"`, …) on the wire.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "filesystem.plugins.HttpMethod")]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

// Typed conversion to the SDK's lean `plugins get` wire shape — drops
// the on-disk-only fields (`cli_zip`, `viewer_zip`). Lets the
// `command::plugins::{get, list}` leaves yield SDK `ResponseManifest`
// items without round-tripping through `serde_json::Value`. The inner
// type parallels (`ViewerRoute`/`ResponseViewerRoute`,
// `McpServer`/`ResponseMcpServer`, `HttpMethod`/`ResponseHttpMethod`)
// are field-identical — collapsing them into shared types is a
// separate cleanup pass. `exec` is the SDK type already.
impl From<Manifest> for objectiveai_sdk::cli::command::plugins::get::ResponseManifest {
    fn from(m: Manifest) -> Self {
        use objectiveai_sdk::cli::command::plugins::get::{
            ResponseHttpMethod, ResponseManifest, ResponseMcpServer,
            ResponseViewerRoute,
        };
        ResponseManifest {
            owner: m.owner,
            name: m.name,
            version: m.version,
            description: m.description,
            exec: m.exec,
            viewer_url: m.viewer_url,
            viewer_routes: m
                .viewer_routes
                .into_iter()
                .map(|r| ResponseViewerRoute {
                    path: r.path,
                    method: match r.method {
                        HttpMethod::Get => ResponseHttpMethod::Get,
                        HttpMethod::Post => ResponseHttpMethod::Post,
                        HttpMethod::Put => ResponseHttpMethod::Put,
                        HttpMethod::Patch => ResponseHttpMethod::Patch,
                        HttpMethod::Delete => ResponseHttpMethod::Delete,
                    },
                    r#type: r.r#type,
                })
                .collect(),
            mcp_servers: m
                .mcp_servers
                .into_iter()
                .map(|s| ResponseMcpServer {
                    name: s.name,
                    authorization: s.authorization,
                })
                .collect(),
        }
    }
}
