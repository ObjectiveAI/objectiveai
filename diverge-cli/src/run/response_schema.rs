//! `response-schema`: the JSON Schema of every line a request can
//! print.

use clap::Args;
use schemars::JsonSchema;

use super::{Emitter, Error};

/// `response-schema` takes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct ResponseSchema {}

/// The JSON Schema of `L`, the one type every line the leaf prints is
/// — the endpoint's own response frame for most, and the command's
/// own line type where the command prints lines of its own — printed
/// as one line, before any jq; its title is `command`, the words that
/// name the request on the command line, and the word `response`.
pub async fn response_schema<L: JsonSchema>(
    command: &str,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let mut schema = schemars::schema_for!(L);
    schema.insert("title".to_owned(), format!("{command} response").into());
    emitter.emit(&schema).await.map(|_| ())
}
