//! Vote type representing a single LLM's selection in a vector completion.

use crate::functions::expression::ToStarlarkValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::AllocDict as StarlarkAllocDict;
use starlark::values::{Heap as StarlarkHeap, Value as StarlarkValue};

/// A single LLM's vote in a vector completion.
///
/// Each LLM in the swarm produces a vote indicating which response(s) it
/// selected. Votes are weighted according to the profile and combined to
/// produce the final scores.
///
/// # Vote Format
///
/// The `vote` field is a vector of decimals corresponding to the responses
/// in the request. Typically one element is 1.0 and the rest are 0.0 (discrete
/// selection), but when `top_logprobs` is used, votes may be probability
/// distributions.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "vector.completions.response.Vote")]
pub struct Vote {
    // --- Identifiers ---
    /// WF-level id of the agent that produced this vote — concatenation
    /// of the primary agent's id with all fallback ids (see
    /// `InlineAgentWithFallbacks::full_id`). Same for every slot in the
    /// same WF request and deterministic across api processes.
    pub agent_full_id: String,
    /// Leaf agent id of the slot that produced this vote (matches the
    /// `agent_id` on the corresponding
    /// [`super::super::super::super::agent::completions::response::unary::AgentCompletion`]).
    /// When fallbacks fired, this is the fallback's id rather than the
    /// primary's.
    pub agent_id: String,
    /// Index of the agent configuration within the swarm.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub swarm_index: u64,
    /// Flattened index accounting for agent counts in the swarm.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub flat_swarm_index: u64,
    /// Content hash of the request messages (for caching/deduplication).
    pub prompt_id: String,
    /// Content hashes of each response option in the request.
    pub responses_ids: Vec<String>,

    // --- Vote data ---
    /// The vote distribution. Each index corresponds to a response from the
    /// request. Typically one element is 1.0 (selected) and the rest are 0.0.
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_rust_decimal)]
    pub vote: Vec<rust_decimal::Decimal>,

    /// The weight applied to this vote when computing final scores.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub weight: rust_decimal::Decimal,

    // --- Source flags ---
    /// If true, this vote was reused from a previous request via the `retry`
    /// parameter. All fields reflect the original request's values.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry: Option<bool>,

    /// If true, this vote was retrieved from cache rather than generated fresh.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,

    // --- Internal ---
    /// Internal index for correlating with completions. Not serialized.
    #[serde(skip)]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub completion_index: Option<u64>,
}

impl ToStarlarkValue for Vote {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        heap.alloc(StarlarkAllocDict([
            ("agent_full_id", self.agent_full_id.to_starlark_value(heap)),
            ("agent_id", self.agent_id.to_starlark_value(heap)),
            ("swarm_index", self.swarm_index.to_starlark_value(heap)),
            (
                "flat_swarm_index",
                self.flat_swarm_index.to_starlark_value(heap),
            ),
            ("prompt_id", self.prompt_id.to_starlark_value(heap)),
            ("responses_ids", self.responses_ids.to_starlark_value(heap)),
            ("vote", self.vote.to_starlark_value(heap)),
            ("weight", self.weight.to_starlark_value(heap)),
            ("retry", self.retry.to_starlark_value(heap)),
            ("from_cache", self.from_cache.to_starlark_value(heap)),
        ]))
    }
}
