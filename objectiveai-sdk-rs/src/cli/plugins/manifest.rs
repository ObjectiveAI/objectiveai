//! The plugin manifest: `objectiveai.json` at the root of a plugin's
//! repo — THE one schema for the one file, read by both halves of the
//! system: the laboratory host (container build: `containerfile`,
//! `port`) and the viewer (surface: `icon`, `tabs`; read from the
//! installed copy at
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/objectiveai.json`).
//!
//! The plugin's IDENTITY is deliberately NOT in the manifest — owner,
//! name, and version come from the PATH / git tag (readers lowercase
//! owner and name). Unknown fields are tolerated (serde's default),
//! so future contract additions pass through unmodeled until typed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A plugin's `objectiveai.json`, whole — ONE schema for the ONE
/// file. The container half (`containerfile`, `port`) is what the
/// laboratory host builds and runs; the viewer half (`icon`, `tabs`)
/// is what the viewer surfaces. Viewer paths are relative to the
/// plugin's `viewer/` folder (authored as if the CWD were inside it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Manifest")]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Repo-relative path (forward slashes) to the Containerfile /
    /// Dockerfile the plugin's image builds from — the repo checkout
    /// is the build context.
    pub containerfile: String,
    /// The port the plugin's MCP server listens on inside the
    /// container — published to a random loopback host port at
    /// create. Never 0.
    pub port: u16,
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
    },
}
