//! The embedding source.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where the agent's vectors come from: an OpenAI-compatible
/// `/embeddings` endpoint, its bearer the vault's.
///
/// A second structure, independent of [`Provider`](super::Provider),
/// because the two are independent facts: the model that speaks may
/// not embed (Anthropic's never does), and a caller pays for vectors
/// separately. Eliza's `plugin-embeddings` is the mechanism — it
/// enables itself on its own URL and deliberately never falls back
/// to the `OPENAI_*` settings.
///
/// HOW THE HARNESS APPLIES IT: `EMBEDDING_BASE_URL`,
/// `EMBEDDING_MODEL`, `EMBEDDING_DIMENSIONS`, and `EMBEDDING_API_KEY`
/// read from the vault under that same key at the start of every
/// run (a vault without it refuses the run) — in the runtime's
/// constructor settings map and the entry process's environment
/// both. ABSENT, the harness sets
/// `ELIZA_DISABLE_LOCAL_EMBEDDINGS=1` and registers no embedding
/// tier at all: Eliza boots, stores every memory WITHOUT a vector,
/// and searches by keyword — an honest degradation the caller chose
/// rather than an on-device embedder they did not.
///
/// The width is stateful: the database keeps one active vector
/// column, and a run whose [`dimensions`](Self::dimensions) differ
/// from the continuation's switches the column and re-embeds every
/// memory in the background — at the caller's cost, the memory
/// surviving. The harness records the width the vectors were built
/// on in the lineage's row, and warns when a run changes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Embedding {
    /// The OpenAI-compatible base URL, `/v1` included.
    pub base_url: String,
    /// The embedding model, in the endpoint's own naming.
    pub model: String,
    /// The vector width the model produces. One of the widths the
    /// database has a column for: 384, 512, 768, 1024, 1536, 2048,
    /// 3072; another is kept on the current column with a warning.
    pub dimensions: u32,
}
