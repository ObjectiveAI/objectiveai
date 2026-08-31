//! Web search and page extraction.

use serde::{Deserialize, Serialize};

/// The `web` switch.
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

/// Web search and page extraction. Every argument is optional
/// because Hermes's keyless ring works with zero credentials — the
/// keys buy quality and quota, not existence. Each is applied by
/// the harness as its same-named env var in the gateway's process
/// environment.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {
    /// Tavily search, applied as `TAVILY_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tavily_api_key: Option<String>,
    /// Exa search, applied as `EXA_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exa_api_key: Option<String>,
    /// Parallel search, applied as `PARALLEL_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_api_key: Option<String>,
    /// Keenable search, applied as `KEENABLE_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keenable_api_key: Option<String>,
    /// Firecrawl extraction, applied as `FIRECRAWL_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firecrawl_api_key: Option<String>,
    /// Brave search, applied as `BRAVE_SEARCH_API_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brave_search_api_key: Option<String>,
    /// A self-hosted SearXNG instance, applied as `SEARXNG_URL`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub searxng_url: Option<String>,
}
