//! The plugin manifest: `objectiveai.json` at the root of a plugin's
//! repo — THE one schema for the one file, read by both halves of the
//! system: the laboratory host (which builds each half's image) and
//! the viewer (surface: `icon`, `tabs`, `scripts`; read from the
//! installed copy at
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/objectiveai.json`).
//!
//! The plugin's IDENTITY is deliberately NOT in the manifest — owner,
//! name, and version come from the PATH / git tag (readers lowercase
//! owner and name). Unknown fields are tolerated (serde's default),
//! so future contract additions pass through unmodeled until typed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The directory a plugin's built viewer assets occupy in the
/// INSTALLED layout — fixed, never declared. The laboratory stages
/// [`Viewer::output`]'s contents into it and the viewer's `plugin://`
/// protocol joins it, so both sides must agree: a mismatch would 404
/// every asset with no error anywhere. A tab `module` of `./home.js`
/// therefore means `<version>/viewer/home.js` on disk.
pub const VIEWER_DIR: &str = "viewer";

/// A plugin's `objectiveai.json`, whole — ONE schema for the ONE
/// file, in two independent halves.
///
/// [`Manifest::mcp`] is the MCP server the laboratory builds and runs;
/// [`Manifest::viewer`] is the extension it builds and the viewer
/// surfaces. Each is OPTIONAL and self-contained: a plugin may ship a
/// server, a viewer extension, or both — but not NEITHER (see
/// [`Manifest::validate`]).
///
/// The SAME file is the repo's manifest and the installed one —
/// nothing rewrites it in between, so every path it declares must
/// describe the BUILT layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Manifest")]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// The MCP-server half. ABSENT = a viewer-only plugin: declaring
    /// it as an agent's plugin fails when the laboratory goes to build
    /// its image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp: Option<Mcp>,
    /// The viewer-extension half. ABSENT = a server-only plugin: it
    /// contributes no tabs, no icon, and `plugin://` serves nothing
    /// for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer: Option<Viewer>,
}

impl Manifest {
    /// A manifest must declare at least one half. Both absent is an
    /// inert plugin — always an authoring mistake, and worth catching
    /// at the parse rather than installing a directory that does
    /// nothing. Every field being optional is also what would let an
    /// unrelated JSON file parse "successfully", so this is the check
    /// that keeps foreign `objectiveai.json`s out.
    pub fn validate(&self) -> Result<(), String> {
        if self.mcp.is_none() && self.viewer.is_none() {
            return Err(
                "plugin manifest declares neither `mcp` nor `viewer` — it would do nothing"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// The MCP-server half: an image the laboratory builds and RUNS,
/// serving MCP on a loopback-published port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Mcp")]
pub struct Mcp {
    /// Repo-relative path (forward slashes) to the Containerfile /
    /// Dockerfile the image builds from. Its OWN DIRECTORY is the
    /// build context — so a Containerfile at `server/Containerfile`
    /// sees `server/` as its root and its `COPY` steps carry no
    /// `server/` prefix.
    pub containerfile: String,
    /// The port the MCP server listens on inside the container —
    /// published to a random loopback host port at create. Never 0.
    pub port: u16,
}

/// The viewer-extension half: an image the laboratory builds and never
/// RUNS — it is created, [`Viewer::output`] is copied out of it, and
/// both the container and the image are discarded. So the build
/// belongs in `RUN` steps, and the plugin owns its whole toolchain
/// (Tailwind, PostCSS, whatever it likes).
///
/// The one contract that build must honor: `react`, `react-dom`,
/// `react/jsx-runtime` and `react-dom/client` must be left EXTERNAL.
/// The host viewer serves them through an import map so every plugin
/// shares its single React instance; a bundle carrying its own copy
/// dies on the first hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Viewer")]
pub struct Viewer {
    /// Repo-relative path (forward slashes) to the Containerfile that
    /// BUILDS the extension. Its OWN DIRECTORY is the build context —
    /// see [`Mcp::containerfile`].
    pub containerfile: String,
    /// Absolute path INSIDE the built image whose CONTENTS are the
    /// built assets. They are copied out to become the installed
    /// [`VIEWER_DIR`], so every path below is relative to THIS
    /// directory's contents: an `output` of `/dist` holding `home.js`
    /// is named `./home.js`. Plugins never produce an archive; the
    /// host packs one.
    pub output: String,
    /// The identity icon, shown beside the identity in the tab strip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub icon: Option<String>,
    /// The plugin's tabs, in declaration order (strip order). A
    /// [`ViewerTab::Channel`] entry (`channel_key`) is a CHANNEL
    /// HANDLER: it never opens at boot — it opens when an offer with
    /// that key is accepted, with the full offer as its arguments,
    /// titled by the offer key. A [`ViewerTab::Tab`] entry (`title`)
    /// is a regular tab, opened at viewer boot. Duplicates (same
    /// `module` among regular tabs, same `channel_key` among
    /// handlers) — entries after the first are ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tabs: Option<Vec<ViewerTab>>,
    /// Scripts the plugin can inject into a browser tab it spawns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub scripts: Option<Vec<ViewerScript>>,
}

/// One injectable script: a self-contained bundle evaluated inside a
/// page the plugin does not own.
///
/// Deliberately NOT shaped like [`ViewerTab`]:
/// - **no `export`** — the injector evaluates a CLASSIC script, which
///   has no module record and therefore nothing to name. A tab module
///   is a value provider (the host needs the component back); a
///   script is an action, and the host has no use for its result.
/// - **no `styles`** — the page is not ours and `plugin://` does not
///   exist there, so a stylesheet URL is unreachable (and a strict
///   site's CSP would block it anyway). A script's CSS must be a
///   string inside its own bundle, applied through CSSOM.
///
/// For the same reason the bundle must inline EVERYTHING, react
/// included: no import map exists in a foreign page, and `import` is
/// a syntax error in a classic script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.ViewerScript")]
pub struct ViewerScript {
    /// How a tab addresses this script when it spawns a browser tab.
    /// Resolving by NAME keeps the runnable set closed: only what the
    /// plugin declared at install time can ever be injected.
    pub name: String,
    /// The built bundle, relative to the installed [`VIEWER_DIR`] —
    /// same rules as a tab's `module`.
    pub module: String,
}

/// One declared viewer tab: the component coordinates the viewer's
/// generic bootstrap dereferences (`import(module)[export]`), plus
/// EXACTLY ONE of `channel_key` (a channel handler, titled by its
/// offer key) or `title` (a regular boot tab). Untagged: the present
/// field decides the variant; an entry carrying both reads as a
/// handler and its `title` is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.ViewerTab")]
#[serde(untagged)]
pub enum ViewerTab {
    /// A channel handler — opened by the accept flow, never at boot.
    #[schemars(title = "Channel")]
    Channel {
        /// The offer key this handler answers (e.g.
        /// `"browser.login"`) — also the opened tab's title.
        channel_key: String,
        /// The component's module path, relative to `viewer/`.
        module: String,
        /// The export holding the component (`None` = `"default"`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        export: Option<String>,
        /// Stylesheets this tab needs, as paths relative to the
        /// `viewer` directory (`"./home.css"`) against the BUILT
        /// layout, like `module`. The viewer injects each as a
        /// `<link rel="stylesheet">` and waits for it BEFORE the
        /// component renders — so there is no flash of unstyled
        /// content, and a sheet that fails to load stops the tab
        /// rather than showing it wrong.
        ///
        /// Declaring them is what makes them checkable: the build
        /// fails if a listed sheet is missing from its output. It is
        /// also the only thing that works — a bundler strips
        /// `import "./x.css"` from a JS entry and emits the file
        /// beside it, so nothing would ever request it.
        ///
        /// Scope is the tab's own document (every tab is its own
        /// webview), so a plugin's global CSS cannot reach another
        /// tab or the chrome.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        styles: Option<Vec<String>>,
    },
    /// A regular tab — opened at viewer boot.
    #[schemars(title = "Tab")]
    Tab {
        /// The tab's display title.
        title: String,
        /// The component's module path, relative to `viewer/`.
        module: String,
        /// The export holding the component (`None` = `"default"`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        export: Option<String>,
        /// Stylesheets this tab needs, as paths relative to the
        /// `viewer` directory (`"./home.css"`) against the BUILT
        /// layout, like `module`. The viewer injects each as a
        /// `<link rel="stylesheet">` and waits for it BEFORE the
        /// component renders — so there is no flash of unstyled
        /// content, and a sheet that fails to load stops the tab
        /// rather than showing it wrong.
        ///
        /// Declaring them is what makes them checkable: the build
        /// fails if a listed sheet is missing from its output. It is
        /// also the only thing that works — a bundler strips
        /// `import "./x.css"` from a JS entry and emits the file
        /// beside it, so nothing would ever request it.
        ///
        /// Scope is the tab's own document (every tab is its own
        /// webview), so a plugin's global CSS cannot reach another
        /// tab or the chrome.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        styles: Option<Vec<String>>,
    },
}
