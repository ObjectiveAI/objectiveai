use crate::functions::inventions::recursive::response;
use crate::agent;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.recursive.response.unary.FunctionInventionRecursive")]
pub struct FunctionInventionRecursive {
    pub id: String,
    pub inventions: Vec<super::FunctionInvention>,
    pub inventions_errors: bool,
    pub created: u64,
    pub object: super::Object,
    pub usage: agent::completions::response::Usage,
}

impl FunctionInventionRecursive {
    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.created = 0;
        for invention in &mut self.inventions {
            invention.inner.normalize_for_tests();
        }

        // sort inventions by JSON representation since ordering is non-deterministic
        self.inventions.sort_by_cached_key(|i| serde_json::to_string(&i.inner).unwrap());

        // re-apply invention indices since indices are non-determinstic
        let mut i = 0;
        for invention in &mut self.inventions {
            invention.index = i;
            i += 1;
        }
    }
}

impl From<response::streaming::FunctionInventionRecursiveChunk>
    for FunctionInventionRecursive
{
    fn from(
        response::streaming::FunctionInventionRecursiveChunk {
            id,
            inventions,
            inventions_errors,
            created,
            object,
            usage,
        }: response::streaming::FunctionInventionRecursiveChunk,
    ) -> Self {
        Self {
            id,
            inventions: inventions
                .into_iter()
                .map(super::FunctionInvention::from)
                .collect(),
            inventions_errors: inventions_errors.unwrap_or(false),
            created,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
