//! The web search toolset.

use serde::{Deserialize, Serialize};

/// `plugin-web-search`: web search through Tavily, the one backend
/// the pinned plugin reads a key for.
///
/// HOW THE HARNESS APPLIES IT: the plugin is listed for the
/// runtime and `TAVILY_API_KEY` is set in its process environment
/// (the plugin reads it as a setting).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Toolset {
    /// The Tavily API key.
    pub tavily_api_key: String,
}
