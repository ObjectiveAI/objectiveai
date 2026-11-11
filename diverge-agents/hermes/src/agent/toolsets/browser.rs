//! Browser automation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Browser automation, with its arguments. The field being absent
/// from [`Toolsets`](super::Toolsets) is Hermes's own default for
/// this toolset; present is the switch thrown on.
///
/// The one slot is optional because the default backend is the
/// container's own headless Chromium, which needs nothing — the
/// slot points the toolset at a browser somewhere else instead,
/// and the empty object is the local browser, coherently.
///
/// SECRET: `BROWSERBASE_API_KEY` and `BROWSERBASE_PROJECT_ID`, both
/// fixed, when the remote is Browserbase; none for a raw CDP
/// endpoint, and none for the local browser.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct Toolset {
    /// The remote backend, one of [`Remote`]'s; absent = the local
    /// headless Chromium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<Remote>,
}

/// One remote browser backend. Untagged — Browserbase is its name
/// as a JSON string, a raw endpoint is an object carrying its
/// address — and a string and an object never collide, so no marker
/// is needed. One slot for both because they are the same role:
/// Browserbase IS a CDP provider, and a request naming a raw
/// endpoint and a managed one would be contradicting itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Remote {
    /// Browserbase, by name. Declares `BROWSERBASE_API_KEY` and
    /// `BROWSERBASE_PROJECT_ID` — the pair is one credential, both
    /// secrets, applied in the gateway's process environment — and
    /// pins `browser.cloud_provider: browserbase`.
    Browserbase(Browserbase),
    /// Any browser reachable over CDP, applied as
    /// `BROWSER_CDP_URL`. No secret.
    Cdp {
        /// The endpoint.
        cdp_url: String,
    },
}

/// [`Remote::Browserbase`]'s name: the one string the slot takes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Browserbase {
    #[default]
    Browserbase,
}
