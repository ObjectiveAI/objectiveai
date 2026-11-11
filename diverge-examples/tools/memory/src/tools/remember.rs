//! `remember`: keep a note.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::memory::State;
use crate::store;

/// The tool's name.
pub const NAME: &str = "remember";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Keep a note, with optional tags to find it by. Answers the note's id.";

/// What `remember` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// What to remember.
    pub text: String,
    /// Tags to find the note by. Absent, none.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Keep the note.
pub async fn call(state: &State, parameters: Parameters) -> Result<Value, Failure> {
    let pool = state.store.pool(&state.client).await?;
    let id = store::notes::add(pool, &parameters.text, &parameters.tags).await?;
    Ok(json!({ "id": id }))
}
