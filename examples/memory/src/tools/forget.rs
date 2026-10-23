//! `forget`: delete a note.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::memory::State;
use crate::store;

/// The tool's name.
pub const NAME: &str = "forget";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Delete a note, by the id `remember` or `recall` gave.";

/// What `forget` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The note, by the id `remember` or `recall` gave.
    pub id: i64,
}

/// Delete the note.
pub async fn call(state: &State, parameters: Parameters) -> Result<Value, Failure> {
    let pool = state.store.pool(&state.client).await?;
    store::notes::forget(pool, parameters.id).await?;
    Ok(json!({ "forgotten": parameters.id }))
}
