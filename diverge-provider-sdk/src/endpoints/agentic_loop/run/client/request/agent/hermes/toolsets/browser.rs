//! Browser automation.

use serde::{Deserialize, Serialize};

/// The `browser` switch.
///
/// Untagged: a bool is the bare switch, an object is the switch
/// thrown on with arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Toolset {
    /// `false` = explicitly off; `true` = on with nothing supplied.
    Switch(bool),
    /// On, with arguments. See [`Config`].
    Config(Config),
}

/// Browser automation. Every argument is optional because the
/// default backend is the container's own headless Chromium, which
/// needs nothing; the arguments point the toolset at a browser
/// somewhere else instead. Applied by the harness as env vars in
/// the gateway's process environment.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {
    /// A remote browser's CDP endpoint, applied as
    /// `BROWSER_CDP_URL`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdp_url: Option<String>,
    /// Browserbase, applied as `BROWSERBASE_API_KEY` — supply
    /// [`browserbase_project_id`](Self::browserbase_project_id)
    /// with it; the pair is one credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browserbase_api_key: Option<String>,
    /// Browserbase's project, applied as
    /// `BROWSERBASE_PROJECT_ID`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browserbase_project_id: Option<String>,
}
