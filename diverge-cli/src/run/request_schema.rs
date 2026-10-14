//! `request-schema`: the JSON Schema of the body a request takes.

use clap::Args;
use schemars::JsonSchema;

use super::{Emitter, Error};

/// `request-schema` takes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct RequestSchema {}

/// The JSON Schema of `Q`, the request's own type, printed as one
/// line; its title is `command`, the words that name the request on
/// the command line, and the word `request`.
pub async fn request_schema<Q: JsonSchema>(
    command: &str,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let mut schema = schemars::schema_for!(Q);
    schema.insert("title".to_owned(), format!("{command} request").into());
    emitter.emit(&schema).await.map(|_| ())
}
