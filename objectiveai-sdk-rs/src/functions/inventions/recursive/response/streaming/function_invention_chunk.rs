use crate::functions;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.recursive.response.streaming.FunctionInventionChunk")]
pub struct FunctionInventionChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner:
        functions::inventions::response::streaming::FunctionInventionChunk,
}

impl FunctionInventionChunk {
    pub fn push(&mut self, other: &FunctionInventionChunk) {
        self.inner.push(&other.inner);
    }

    /// Produces log files for this invention within a recursive invention.
    ///
    /// Returns `(reference, files)` where `reference` includes `"index"`.
    /// Files are written under `functions/inventions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> (serde_json::Value, Vec<crate::filesystem::logs::LogFile>) {
        let (mut reference, files) = match self.inner.produce_files() {
            Some((reference, files)) => (reference, files),
            None => return (serde_json::json!({ "type": "reference", "index": self.index }), Vec::new()),
        };
        if let Some(map) = reference.as_object_mut() {
            map.insert("index".to_string(), serde_json::json!(self.index));
        }
        (reference, files)
    }
}
