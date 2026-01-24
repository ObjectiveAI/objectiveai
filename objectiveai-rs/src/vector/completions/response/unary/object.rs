//! Object type marker for unary vector completion responses.

use crate::vector::completions::response;
use serde::{Deserialize, Serialize};

/// Object type for unary vector completion responses.
///
/// Serializes to `"vector.completion"` in JSON.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
    /// A complete vector completion response.
    #[serde(rename = "vector.completion")]
    #[default]
    VectorCompletion,
}

impl From<response::streaming::Object> for Object {
    fn from(object: response::streaming::Object) -> Self {
        match object {
            response::streaming::Object::VectorCompletionChunk => {
                Object::VectorCompletion
            }
        }
    }
}
