use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Platform;

/// Declarative metadata a plugin ships with itself. The wire shape is
/// JSON; the on-disk convention (sibling file, embedded resource,
/// `--manifest` flag, …) is deliberately out of scope of this struct
/// and will be settled in a follow-up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.plugins.Manifest")]
pub struct Manifest {
    /// One-line description of what the plugin does. Surfaced in
    /// listings and the plugin's `--help`-equivalent UI.
    pub description: String,

    /// Version string. Semver convention is recommended but not
    /// enforced — the host just displays whatever's here.
    pub version: String,

    /// GitHub `<owner>` segment of the source repo. Authors write
    /// their canonical owner here; the installer overwrites this
    /// field with whatever owner it was actually installed from (so
    /// forks land on disk with the fork's owner, not the upstream's).
    pub owner: String,

    /// Author or authors of the plugin. Free-form string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub author: Option<String>,

    /// Homepage or repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub homepage: Option<String>,

    /// SPDX license identifier (or any string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub license: Option<String>,

    /// Release-asset filename per platform — what the cli should
    /// download from the GitHub release tagged `v<version>` to install
    /// the plugin's binary on each platform. Values are filenames
    /// (e.g. `psyops-linux-x86_64`, `psyops-windows-x86_64.exe`), NOT
    /// URLs; the URL is composed from the repository + tag + asset
    /// name elsewhere.
    ///
    /// **Every platform field is optional.** Declare entries only for
    /// the platforms this plugin actually ships a binary for; absent
    /// platforms are simply not supported by this release. A plugin
    /// shipping only Linux x86_64 declares one entry; a plugin
    /// shipping all six declares six. All-None ↔ field omitted in
    /// the wire shape.
    #[serde(default, skip_serializing_if = "Binaries::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub binaries: Binaries,

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
    /// and [`Self::mobile_ready`] apply to remote-URL viewers the same
    /// way they apply to zip-bundled viewers — the embedded axum
    /// server still hosts the declared routes; the iframe still
    /// receives the same postMessage protocol regardless of where
    /// its HTML/JS loaded from.
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

    /// Plugin author opts in to mobile viewer support by setting
    /// this. Mobile viewer builds only surface plugins with this
    /// flag true — mobile has no local backend binary, so plugin
    /// UIs that require a backend will misbehave unless their
    /// authors specifically design for "no-backend" mode. Defaults
    /// to false (desktop-only).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mobile_ready: bool,

    /// MCP servers the plugin wants the host to expose. Each entry
    /// has a `name` (the identifier agents reference via
    /// [`objectiveai_sdk::agent::ClientObjectiveaiMcpPluginEntry::mcp_servers`])
    /// plus the same `url` + `authorization` shape
    /// [`objectiveai_sdk::agent::McpServer`] uses. Auth-requiring entries flag
    /// `authorization = true`; credentials are resolved by the host
    /// (env vars / config), not the manifest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Vec<McpServer>,
}

impl Manifest {
    /// Whether this plugin presents a viewer tab in the host. True
    /// iff either viewer source field — [`Self::viewer_zip`] or
    /// [`Self::viewer_url`] — is set.
    pub fn has_viewer(&self) -> bool {
        self.viewer_zip.is_some() || self.viewer_url.is_some()
    }

    /// LLM-visible tool name. See
    /// [`objectiveai_sdk::agent::materialize_tool_name`].
    pub fn tool_name(&self, name: &str) -> String {
        objectiveai_sdk::agent::materialize_tool_name(&self.owner, name, &self.version)
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
        // Each MCP-server entry: non-empty name + url; the list as a
        // whole must have no `name` duplicates (since agents reference
        // by name) AND no `url` duplicates (no point declaring two
        // entries pointing at the same upstream).
        for entry in &self.mcp_servers {
            if entry.name.is_empty() {
                return Err("mcp_servers[i].name cannot be empty");
            }
            if entry.url.is_empty() {
                return Err("mcp_servers[i].url cannot be empty");
            }
        }
        for (i, a) in self.mcp_servers.iter().enumerate() {
            for b in &self.mcp_servers[i + 1..] {
                if a.name == b.name {
                    return Err("mcp_servers contains duplicate name");
                }
                if a.url == b.url {
                    return Err("mcp_servers contains duplicate url");
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

/// Release-asset filename per platform. Every field is optional;
/// declare only the platforms a plugin ships for. The wire shape is
/// a flat JSON object — absent platforms are omitted, never
/// serialised as `null`.
///
/// Exposes a [`Self::get`] method that takes a [`Platform`] enum so
/// callers can read the asset filename for the current host without
/// pattern-matching the field set themselves.
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq,
)]
#[schemars(rename = "filesystem.plugins.Binaries")]
pub struct Binaries {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub linux_x86_64: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub linux_aarch64: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub windows_x86_64: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub windows_aarch64: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub macos_x86_64: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub macos_aarch64: Option<String>,
}

impl Binaries {
    /// Asset filename for the given platform, if declared.
    pub fn get(&self, platform: Platform) -> Option<&String> {
        match platform {
            Platform::LinuxX86_64 => self.linux_x86_64.as_ref(),
            Platform::LinuxAarch64 => self.linux_aarch64.as_ref(),
            Platform::WindowsX86_64 => self.windows_x86_64.as_ref(),
            Platform::WindowsAarch64 => self.windows_aarch64.as_ref(),
            Platform::MacosX86_64 => self.macos_x86_64.as_ref(),
            Platform::MacosAarch64 => self.macos_aarch64.as_ref(),
        }
    }

    /// True when no platform has an asset declared.
    pub fn is_empty(&self) -> bool {
        self.linux_x86_64.is_none()
            && self.linux_aarch64.is_none()
            && self.windows_x86_64.is_none()
            && self.windows_aarch64.is_none()
            && self.macos_x86_64.is_none()
            && self.macos_aarch64.is_none()
    }

    /// Count of declared platforms.
    pub fn len(&self) -> usize {
        [
            &self.linux_x86_64,
            &self.linux_aarch64,
            &self.windows_x86_64,
            &self.windows_aarch64,
            &self.macos_x86_64,
            &self.macos_aarch64,
        ]
        .iter()
        .filter(|o| o.is_some())
        .count()
    }
}

/// MCP server entry inside [`Manifest::mcp_servers`]. Same `url` +
/// `authorization` semantics as [`objectiveai_sdk::agent::McpServer`] plus a
/// `name` field that agent declarations reference (via
/// [`objectiveai_sdk::agent::ClientObjectiveaiMcpPluginEntry::mcp_servers`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "filesystem.plugins.McpServer")]
pub struct McpServer {
    /// Author-chosen identifier. Unique per plugin manifest. Agents
    /// declare which subset of the plugin's MCP servers they want
    /// exposed by listing names here.
    pub name: String,
    /// Upstream MCP server URL.
    pub url: String,
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

/// A [`Manifest`] enriched with the plugin's identifying `name` and
/// the `source` it was loaded from. Used when listing or describing
/// installed plugins, where the bare manifest fields are not enough
/// to identify which plugin they belong to or where they came from.
///
/// `name` sits before the manifest body; `source` sits after. The
/// `manifest` field is `#[serde(flatten)]`-ed so the wire shape is
/// one flat JSON object — `serde_json`'s `preserve_order` feature
/// keeps the declared field order, so consumers see `name` first
/// and `source` last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.plugins.ManifestWithNameAndSource")]
pub struct ManifestWithNameAndSource {
    /// The plugin's identifier — the filename it lives under in the
    /// plugins directory (e.g. `psyops` for `~/.objectiveai/plugins/psyops`).
    pub name: String,
    #[serde(flatten)]
    pub manifest: Manifest,
    /// Where this manifest came from — e.g. an absolute filesystem path,
    /// a URL, or a registry reference. Free-form string; the host
    /// just displays it.
    pub source: String,
}

impl ManifestWithNameAndSource {
    /// LLM-visible tool name. See [`Manifest::tool_name`] — this
    /// helper supplies the `name` field automatically.
    pub fn tool_name(&self) -> String {
        self.manifest.tool_name(&self.name)
    }
}
