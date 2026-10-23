//! `recall`: find notes again.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::memory::State;
use crate::store;

/// The tool's name.
pub const NAME: &str = "recall";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Find notes: those carrying every tag given, and, with a query, only those whose text matches it, best \
match first. Without a query, the newest first. Each with its id, text, tags and when it was kept.";

/// How many notes a recall shows when the caller does not say.
const LIMIT: u32 = 20;

/// What `recall` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Words to search the notes' text for, as a web search reads
    /// them: quoted phrases, `or`, and `-` to exclude. Absent, every
    /// note, newest first.
    #[serde(default)]
    pub query: Option<String>,
    /// Only notes carrying every one of these tags. Absent, any.
    #[serde(default)]
    pub tags: Vec<String>,
    /// At most this many notes. Absent, 20.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// The notes found.
pub async fn call(state: &State, parameters: Parameters) -> Result<Value, Failure> {
    let pool = state.store.pool(&state.client).await?;
    let limit = i64::from(parameters.limit.unwrap_or(LIMIT));
    let notes = store::notes::recall(pool, parameters.query.as_deref(), &parameters.tags, limit).await?;
    Ok(json!({ "notes": notes }))
}
