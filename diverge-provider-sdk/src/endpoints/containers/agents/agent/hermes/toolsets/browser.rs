//! Browser automation.

use serde::{Deserialize, Serialize};

/// Browser automation, with its arguments. The field being absent
/// from [`Toolsets`](super::Toolsets) is Hermes's own default for
/// this toolset; present is the switch thrown on.
///
/// The one slot is optional because the default backend is the
/// container's own headless Chromium, which needs nothing — the
/// slot points the toolset at a browser somewhere else instead,
/// and the empty object is the local browser, coherently.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
    /// The remote backend, one of [`Remote`]'s; absent = the local
    /// headless Chromium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<Remote>,
}

/// One remote browser backend, named by the credentials it takes.
/// Untagged — the variants' field sets are disjoint, so the fields
/// are the discriminator and no marker is needed. One slot for
/// both because they are the same role: Browserbase IS a CDP
/// provider, and a request naming a raw endpoint and a managed one
/// would be contradicting itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Remote {
    /// Any browser reachable over CDP, applied as
    /// `BROWSER_CDP_URL`.
    Cdp {
        /// The endpoint.
        cdp_url: String,
    },
    /// Browserbase — the pair is one credential, and the type now
    /// says so. Applied as `BROWSERBASE_API_KEY` and
    /// `BROWSERBASE_PROJECT_ID`.
    Browserbase {
        /// The key.
        browserbase_api_key: String,
        /// The project.
        browserbase_project_id: String,
    },
}
