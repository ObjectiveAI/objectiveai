//! The inference source.

use serde::{Deserialize, Serialize};

/// Where the agent's text models come from: one OpenAI-compatible
/// endpoint, with its credential as an argument.
///
/// One structure rather than one per upstream because ONE plugin
/// is the honest path: Eliza's `plugin-openai` is the only provider
/// in the pinned tree that takes a base URL and registers every text
/// tier (nano through mega, the response handler, the planner) plus
/// the tokenizer in one place — the path its own docs name for
/// "OpenRouter, Cerebras, local servers". Its wire-specific siblings
/// (Anthropic, Google, Ollama) take no base URL, and the Diverge
/// relay and every upstream this protocol routes speak this wire.
///
/// HOW THE HARNESS APPLIES IT: `OPENAI_BASE_URL`, `OPENAI_API_KEY`
/// in the runtime's process environment (the key's presence is also
/// what enables the plugin), and `OPENAI_NANO_MODEL`,
/// `OPENAI_SMALL_MODEL`, `OPENAI_MEDIUM_MODEL`, `OPENAI_LARGE_MODEL`,
/// `OPENAI_MEGA_MODEL` ALL set to [`model`](Self::model) — Eliza's
/// tier ladder collapsed to the one model the caller named, so its
/// fallback chains never surprise. Embeddings are NOT this
/// structure's: `plugin-openai`'s own embedding tier is left
/// unregistered, and [`Embedding`](super::Embedding) is the one
/// source of vectors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provider {
    /// The OpenAI-compatible base URL, `/v1` included.
    pub base_url: String,
    /// The bearer the endpoint takes. Any non-empty value: the
    /// plugin only warns when it is missing, but an endpoint that
    /// checks it fails every call.
    pub api_key: String,
    /// The model, in the endpoint's own naming — every tier.
    pub model: String,
}
