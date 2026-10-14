//! What is beneath a request: `request-schema`, the JSON Schema of
//! the body it takes.

use clap::{Args, Subcommand};
use schemars::JsonSchema;

use super::{Emitter, Error};

/// `request-schema` takes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct RequestSchema {}

/// The subcommands beneath a request that takes a body — every
/// request but `postgres get` and `postgres list`. Given, the request
/// is not put and the daemon is not dialled: the subcommand answers
/// from the SDK alone.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Below {
    /// The JSON Schema of the body the request takes, as the SDK
    /// states it: one compact line, through `--jq` when one is given.
    RequestSchema(RequestSchema),
}

/// The JSON Schema of `Q`, the request's own type, printed as one
/// line; its title is `command`, the words that name the request on
/// the command line.
pub async fn request_schema<Q: JsonSchema>(
    command: &str,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let mut schema = schemars::schema_for!(Q);
    schema.insert("title".to_owned(), command.into());
    emitter.emit(&schema).await.map(|_| ())
}
