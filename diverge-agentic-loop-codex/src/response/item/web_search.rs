//! The `web_search` item: a search the model made.

use serde::Deserialize;

/// A web search — Codex's own, never one of the caller's tools:
/// started when kicked off, completed when the results are returned
/// to the agent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WebSearch {
    /// The Responses API's id for the search call.
    pub id: String,
    /// The query, as the model phrased it.
    pub query: String,
    /// What the search did.
    pub action: WebSearchAction,
}

/// What a web search did: the Responses API's own action, tagged
/// `type`. The source carries `#[serde(other)]` on its last variant
/// — the one open tail on this wire, and the wire's own, so a newer
/// action still parses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchAction {
    /// A search.
    Search {
        /// The query.
        #[serde(default)]
        query: Option<String>,
        /// The queries, when there were several.
        #[serde(default)]
        queries: Option<Vec<String>>,
    },
    /// A page opened.
    OpenPage {
        /// Its URL.
        #[serde(default)]
        url: Option<String>,
    },
    /// A pattern looked for in a page.
    FindInPage {
        /// The page's URL.
        #[serde(default)]
        url: Option<String>,
        /// The pattern.
        #[serde(default)]
        pattern: Option<String>,
    },
    /// An action newer than the pin.
    #[serde(other)]
    Other,
}
