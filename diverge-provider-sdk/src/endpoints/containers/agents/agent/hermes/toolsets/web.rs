//! Web search and page extraction.

use serde::{Deserialize, Serialize};

/// Web search and page extraction, with its arguments. The field
/// being absent from [`Toolsets`](super::Toolsets) is Hermes's own
/// default for this toolset; present is the switch thrown on.
///
/// Two roles, two slots: [`search`](Self::search) is what answers
/// queries, [`extract`](Self::extract) is what turns a page into
/// text — different tools, different backends, independently
/// keyed. Each slot names ONE backend and carries its credential;
/// a slot left absent runs on Hermes's keyless free-tier ring,
/// which needs nothing — the keys buy quality and quota, not
/// existence. Every combination is coherent, the empty object
/// included.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
    /// The search backend, one of [`Search`]'s; absent = the
    /// keyless ring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<Search>,
    /// The extraction backend, one of [`Extract`]'s; absent = the
    /// keyless ring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<Extract>,
}

/// One search backend, named by the credential it takes. Untagged —
/// every variant carries exactly one distinctly-named field, so the
/// field name is the discriminator and no marker is needed.
///
/// One backend, deliberately: Hermes can pool several keys as
/// fallbacks for each other, but a vocabulary where extra keys
/// quietly change behavior is the environment-dependent meaning
/// this module keeps rejecting. The caller names the backend; the
/// keyless ring remains the fallback story. The harness applies
/// the variant's env var in the gateway's process environment and
/// pins `web.search_backend` to the named provider in the config
/// it owns.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Search {
    /// Tavily, applied as `TAVILY_API_KEY`.
    Tavily {
        /// The key.
        tavily_api_key: String,
    },
    /// Exa, applied as `EXA_API_KEY`.
    Exa {
        /// The key.
        exa_api_key: String,
    },
    /// Parallel, applied as `PARALLEL_API_KEY`.
    Parallel {
        /// The key.
        parallel_api_key: String,
    },
    /// Keenable, applied as `KEENABLE_API_KEY`.
    Keenable {
        /// The key.
        keenable_api_key: String,
    },
    /// Brave, applied as `BRAVE_SEARCH_API_KEY`.
    Brave {
        /// The key.
        brave_search_api_key: String,
    },
    /// A self-hosted SearXNG instance, applied as `SEARXNG_URL` —
    /// the one backend whose credential is an address.
    Searxng {
        /// The instance.
        searxng_url: String,
    },
}

/// One extraction backend, named by the credential it takes —
/// [`Search`]'s discipline exactly, single-variant today because
/// Hermes has one keyed extractor at this pin. The harness applies
/// the env var and pins `web.extract_backend`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Extract {
    /// Firecrawl, applied as `FIRECRAWL_API_KEY`.
    Firecrawl {
        /// The key.
        firecrawl_api_key: String,
    },
}
