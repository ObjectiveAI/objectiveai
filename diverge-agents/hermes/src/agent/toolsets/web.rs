//! Web search and page extraction.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Web search and page extraction, with its arguments. The field
/// being absent from [`Toolsets`](super::Toolsets) is Hermes's own
/// default for this toolset; present is the switch thrown on.
///
/// Two roles, two slots: [`search`](Self::search) is what answers
/// queries, [`extract`](Self::extract) is what turns a page into
/// text — different tools, different backends, independently
/// keyed. Each slot names ONE backend, and naming a keyed one
/// declares its secret; a slot left absent runs on Hermes's keyless
/// free-tier ring, which needs nothing — the keys buy quality and
/// quota, not existence. Every combination is coherent, the empty
/// object included.
///
/// SECRET: the named search backend's, fixed — `TAVILY_API_KEY`,
/// `EXA_API_KEY`, `PARALLEL_API_KEY`, `KEENABLE_API_KEY` or
/// `BRAVE_SEARCH_API_KEY` for [`Keyed`]'s five, none for SearXNG;
/// and `FIRECRAWL_API_KEY`, fixed, for [`Extract::Firecrawl`].
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
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

/// One search backend. Untagged — a keyed backend is its name as a
/// JSON string, a self-hosted SearXNG is an object carrying its
/// address — and a string and an object never collide, so no marker
/// is needed.
///
/// One backend, deliberately: Hermes can pool several keys as
/// fallbacks for each other, but a vocabulary where extra keys
/// quietly change behavior is the environment-dependent meaning
/// this module keeps rejecting. The caller names the backend; the
/// keyless ring remains the fallback story. The harness applies the
/// keyed backend's secret in the gateway's process environment, or
/// SearXNG's address, and pins `web.search_backend` to the named
/// provider in the config it owns.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Search {
    /// A keyed backend, by name; its key is the secret [`Keyed`]
    /// names.
    Keyed(Keyed),
    /// A self-hosted SearXNG instance, applied as `SEARXNG_URL` —
    /// the one backend whose argument is an address, and no secret.
    Searxng {
        /// The instance.
        searxng_url: String,
    },
}

/// The keyed search backends, by Hermes's own plugin names.
/// Kebab-case on the wire.
///
/// SECRET: one per variant, fixed, applied in the gateway's process
/// environment under its own name — as each variant says.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Keyed {
    /// Tavily. Declares `TAVILY_API_KEY`.
    Tavily,
    /// Exa. Declares `EXA_API_KEY`.
    Exa,
    /// Parallel. Declares `PARALLEL_API_KEY`.
    Parallel,
    /// Keenable. Declares `KEENABLE_API_KEY`.
    Keenable,
    /// Brave, on its free plan. Declares `BRAVE_SEARCH_API_KEY`.
    BraveFree,
}

/// One extraction backend, by Hermes's own plugin name —
/// single-variant today because Hermes has one keyed extractor at
/// this pin. Kebab-case on the wire. The harness applies the secret
/// and pins `web.extract_backend`.
///
/// SECRET: `FIRECRAWL_API_KEY`, fixed, for [`Firecrawl`](Self::Firecrawl).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Extract {
    /// Firecrawl. Declares `FIRECRAWL_API_KEY`.
    Firecrawl,
}
