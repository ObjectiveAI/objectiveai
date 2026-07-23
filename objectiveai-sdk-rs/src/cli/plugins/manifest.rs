//! The installed-plugin manifest: `objectiveai.json` at an installed
//! plugin's version root —
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/objectiveai.json`.
//!
//! The plugin's IDENTITY is deliberately NOT in the manifest — owner,
//! name, and version come from the PATH (readers lowercase owner and
//! name). Unknown fields are tolerated (serde's default), so the
//! parts of the install contract that have no Rust consumers yet
//! (`exec`, `cli_zip`, `mcp_servers`, …) pass through unmodeled until
//! the plugins-install work types them.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An installed plugin's `objectiveai.json` — the typed slice of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Manifest")]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// The plugin's viewer surface — ABSENT for plugins with no UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub viewer: Option<Viewer>,
}

/// A plugin's viewer surface: the identity icon and the tabs it
/// contributes. Every path is relative to the plugin's `viewer/`
/// folder (authored as if the CWD were inside it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Viewer")]
pub struct Viewer {
    /// The identity icon, shown beside the identity in the tab strip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub icon: Option<String>,
    /// The plugin's tabs, by NAME — manifest order is strip order.
    /// Every declared tab opens at viewer boot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tabs: Option<IndexMap<String, ViewerTab>>,
}

/// One declared viewer tab: the component coordinates the viewer's
/// generic bootstrap dereferences (`import(module)[export]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.ViewerTab")]
pub struct ViewerTab {
    /// The component's module path, relative to `viewer/`.
    pub module: String,
    /// The export holding the component (`None` = `"default"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub export: Option<String>,
    /// Display title (`None` = the tab's name key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub title: Option<String>,
}
