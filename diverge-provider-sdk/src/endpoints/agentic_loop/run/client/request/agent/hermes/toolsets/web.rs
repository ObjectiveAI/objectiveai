//! Web search and page extraction.

use serde::{Deserialize, Serialize};

/// Web search and page extraction, with its arguments. The field
/// being absent from [`Toolsets`](super::Toolsets) is Hermes's own
/// default for this toolset; present is the switch thrown on.
///
/// Every argument is optional because Hermes's keyless ring works
/// with zero credentials — the keys buy quality and quota, not
/// existence. Each is applied by the harness as its same-named env
/// var in the gateway's process environment.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
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
