use std::borrow::Cow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;

/// An inner error from a [`FunctionInventionRecursiveChunk`](super::FunctionInventionRecursiveChunk).
///
/// Always carries `function_invention_index` identifying which wrapped
/// non-recursive `FunctionInventionChunk` produced the error (matches
/// [`FunctionInventionChunk::index`](super::FunctionInventionChunk::index)).
///
/// The optional `agent_completion_index` disambiguates the failure site:
///
/// - `None` → the wrapped invention's own top-level `.error` (the invention
///   itself failed).
/// - `Some(N)` → an inner error from agent completion `N` inside the
///   wrapped invention (one of the items the non-recursive invention's
///   own `inner_errors()` would have yielded).
///
/// Wire shape:
/// ```json
/// { "function_invention_index": 0, "error": { } }
/// { "function_invention_index": 0, "agent_completion_index": 2, "error": { } }
/// ```
///
/// `agent_completion_index` is omitted on the wire when `None`,
/// matching the chunk-format conventions in the rest of the SDK.
///
/// The recursive chunk has no top-level `.error` field of its own,
/// so there is nothing to exclude at this level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.inventions.recursive.response.streaming.InnerError"
)]
pub struct InnerError<'a> {
    /// Index of the wrapped non-recursive `FunctionInventionChunk`.
    pub function_invention_index: u64,
    /// `Some(N)` if the error came from agent completion `N` inside the
    /// wrapped invention; `None` if the error is the wrapped invention's
    /// own top-level `.error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_completion_index: Option<u64>,
    /// The underlying error.
    pub error: Cow<'a, error::ResponseError>,
}
