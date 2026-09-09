//! The inference source.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where the agent's text models come from: one OpenAI-compatible
/// endpoint, its bearer the vault's.
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
/// HOW THE HARNESS APPLIES IT: `OPENAI_BASE_URL`, and
/// `OPENAI_API_KEY` read from the vault under that same key at the
/// start of every run (the key's presence is also what enables the
/// plugin; a vault without it refuses the run), and
/// `OPENAI_NANO_MODEL`, `OPENAI_SMALL_MODEL`, `OPENAI_MEDIUM_MODEL`,
/// `OPENAI_LARGE_MODEL`, `OPENAI_MEGA_MODEL` ALL set to
/// [`model`](Self::model) — Eliza's tier ladder collapsed to the one
/// model the caller named, so its fallback chains never surprise.
/// Every setting goes in the runtime's constructor settings map AND
/// the entry process's environment: the core's `getSetting` never
/// reads the environment, and this plugin's own shim does. Embeddings are NOT this
/// structure's: `plugin-openai`'s own embedding tier is left
/// unregistered, and [`Embedding`](super::Embedding) is the one
/// source of vectors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The OpenAI-compatible base URL, `/v1` included.
    pub base_url: String,
    /// The model, in the endpoint's own naming — every tier.
    pub model: String,
}
